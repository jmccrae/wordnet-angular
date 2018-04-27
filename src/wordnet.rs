//! Functions for handling the in-memory model of WordNet and loading it form
//! disk
//use glosstag::{Gloss,build_glosstags};
use std::collections::HashMap;
use links::{Link,LinkType};
use serde_json;
use rusqlite;
use wordnet_model::{Synset};

pub type WNKey=String;


///// A WordNet Key consisting of 8 digits and a part of speech.
///// This data structure stores the value as a 4-byte integer to save memory
//#[derive(Clone,Debug,PartialEq,Eq,Hash,PartialOrd)]
//pub struct WNKey(u32);
//
////impl WNKey {
////    /// Create from an ID and a part of speech
////    pub fn new(id : u32, pos : char) -> Result<WNKey, WordNetLoadError> {
////        match pos {
////            'n' => Ok(WNKey((id << 8) + 1)),
////            'v' => Ok(WNKey((id << 8) + 2)),
////            'a' => Ok(WNKey((id << 8) + 3)),
////            'r' => Ok(WNKey((id << 8) + 4)),
////            's' => Ok(WNKey((id << 8) + 5)),
////            'p' => Ok(WNKey((id << 8) + 6)),
////            'x' => Ok(WNKey((id << 8) + 7)),
////            _ => Err(WordNetLoadError::BadKey(format!("Bad WN POS: {}", pos)))
////        }
////    }
////        
////    pub fn from_slice(slice : &[u8]) -> Result<WNKey, WordNetLoadError> {
////        let s = String::from_utf8(Vec::from(slice))
////            .map_err(|_| WordNetLoadError::BadKey(format!("Invalid UTF-8")))?;
////        WNKey::from_str(&s)
////    }
////}
//
//impl FromStr  for WNKey {
//    type Err = WordNetLoadError;
//    fn from_str(s : &str) -> Result<WNKey, WordNetLoadError> { 
//        if s.len() != 10 {
//            Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
//        } else {
//            let num = u32::from_str(&s.chars().take(8).collect::<String>())
//                .map_err(|_| WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))? << 8;
//            match s.chars().skip(9).next() {
//                Some('n') => Ok(WNKey(0x00000001 | num)),
//                Some('v') => Ok(WNKey(0x00000002 | num)),
//                Some('a') => Ok(WNKey(0x00000003 | num)),
//                Some('r') => Ok(WNKey(0x00000004 | num)),
//                Some('s') => Ok(WNKey(0x00000005 | num)),
//                Some('p') => Ok(WNKey(0x00000006 | num)),
//                Some('x') => Ok(WNKey(0x00000007 | num)),
//                _ => Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
//            }
//        }
//    }
//}
//
//impl ToString for WNKey {
//    fn to_string(&self) -> String { 
//        match self.0 & 0x0000000f {
//            1 => format!("{:08}-n", (self.0 & 0xfffffff0) >> 8),
//            2 => format!("{:08}-v", (self.0 & 0xfffffff0) >> 8),
//            3 => format!("{:08}-a", (self.0 & 0xfffffff0) >> 8),
//            4 => format!("{:08}-r", (self.0 & 0xfffffff0) >> 8),
//            5 => format!("{:08}-s", (self.0 & 0xfffffff0) >> 8),
//            6 => format!("{:08}-p", (self.0 & 0xfffffff0) >> 8),
//            7 => format!("{:08}-x", (self.0 & 0xfffffff0) >> 8),
//            _ => format!("{:08}-?", (self.0 & 0xfffffff0) >> 8)
//        }
//    }
//}
//
//impl Serialize for WNKey {
//    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//        where S: Serializer {
//        serializer.serialize_str(&self.to_string())
//    }
//}
//
//impl<'de> Deserialize<'de> for WNKey {
//    fn deserialize<D>(deserializer: D) -> Result<WNKey, D::Error>
//        where D: Deserializer<'de> {
//        deserializer.deserialize_str(WNKeyVisitor)        
//    }
//}
//
//struct WNKeyVisitor;
//
//impl<'de> Visitor<'de> for WNKeyVisitor {
//    type Value = WNKey;
//
//    fn expecting(&self, formatter : &mut Formatter) -> FormatResult {
//        formatter.write_str("A WordNet key such as 00001740-a")
//    }
//
//    fn visit_str<E>(self, value : &str) -> Result<WNKey, E>  where E : DeError {
//        WNKey::from_str(value)
//            .map_err(|e| E::custom(e))
//    }
//
//    fn visit_string<E>(self, value : String) -> Result<WNKey, E> where E : DeError {
//        WNKey::from_str(&value)
//            .map_err(|e| E::custom(e))
//    }
//}

fn sqlite_query_opt_map<F,A,E>(query : &str, values : &[&rusqlite::types::ToSql],
                             foo : F) -> Result<Option<A>,WordNetLoadError> 
                    where F: FnOnce(String) -> Result<A,E>,
                          WordNetLoadError : From<E> {
                        
    let conn = WordNet::open_conn()?;
    let mut stmt = conn.prepare(query)?;
    let mut res = stmt.query(values)?;
    match res.next() {
        Some(res) => {
            Ok(Some(foo(res?.get(0))?))
        },
        None => Ok(None)
    }
}

fn sqlite_query_vec<F,A,E>(query : &str, values : &[&rusqlite::types::ToSql],
                           foo : F) -> Result<Vec<A>,WordNetLoadError> 
                    where F: Fn(String) -> Result<A,E>,
                          WordNetLoadError : From<E> {
                        
    let conn = WordNet::open_conn()?;
    let mut stmt = conn.prepare(query)?;
    let mut res = stmt.query(values)?;
    let mut data = Vec::new();
    while let Some(r) = res.next() {
        data.push(foo(r?.get(0))?);
    }
    Ok(data)
}

pub struct WordNetBuilder { 
    conn: rusqlite::Connection,
    synsets : HashMap<WNKey, Synset>,
    by_ili : HashMap<String, WNKey>,
    by_pwn30 : HashMap<WNKey, WNKey>,
    by_pwn20 : HashMap<WNKey, WNKey>,
    by_sense_key : HashMap<String, WNKey>
}

pub struct WordNet;

fn ok_wordnet_str(s : String) -> Result<String, WordNetLoadError> {
    Ok(s)
}

impl WordNetBuilder {
    pub fn new() -> Result<WordNetBuilder,WordNetLoadError> {
        let conn = WordNet::open_conn()?;
        conn.execute("CREATE TABLE synsets (
                      key TEXT NOT NULL,
                      ili TEXT NOT NULL,
                      json TEXT NOT NULL)", &[])?;
        conn.execute("CREATE INDEX synsets_key ON synsets (key)", &[])?;
        conn.execute("CREATE INDEX synsets_ili ON synsets (ili)", &[])?;
        conn.execute("CREATE TABLE lemmas (
                      lemma TEXT NOT NULL,
                      form TEXT NOT NULL,
                      language TEXT NOT NULL,
                      synset TEXT NOT NULL,
                      FOREIGN KEY (synset) REFERENCES synsets (key))", &[])?;
        conn.execute("CREATE INDEX lemmas_form ON lemmas (form, language)", &[])?;
        conn.execute("CREATE INDEX lemmas_synset ON lemmas (synset)", &[])?;
        conn.execute("CREATE TABLE sense_keys (
                      sense_key TEXT NOT NULL,
                      synset TEXT NOT NULL,
                      FOREIGN KEY (synset) REFERENCES synsets (key))", &[])?;
        conn.execute("CREATE INDEX sense_keys_sense_key ON sense_keys (sense_key)", &[])?;
        conn.execute("CREATE INDEX sense_keys_synset ON sense_keys (synset)", &[])?;
        conn.execute("CREATE TABLE links (
                      synset TEXT NOT NULL,
                      type TEXT NOT NULL,
                      target TEXT NOT NULL,
                      FOREIGN KEY (synset) REFERENCES synsets (key))", &[])?;
        conn.execute("CREATE INDEX links_synset ON links (synset)", &[])?;
        conn.execute("CREATE TABLE old_keys (
                      idx TEXT NOT NULL,
                      key TEXT NOT NULL,
                      synset TEXT NOT NULL,
                      FOREIGN KEY (synset) REFERENCES synsets (key))", &[])?;
        conn.execute("CREATE INDEX old_keys_idx ON old_keys (idx)", &[])?;
        conn.execute("CREATE INDEX old_keys_key ON old_keys (key)", &[])?;
        conn.execute("CREATE INDEX old_keys_synset ON old_keys (synset)", &[])?;
        Ok(WordNetBuilder { 
            conn : conn,
            synsets : HashMap::new(),
            by_ili : HashMap::new(),
            by_pwn30: HashMap::new(),
            by_pwn20: HashMap::new(),
            by_sense_key : HashMap::new()
        })
    }

    pub fn set_synsets(&mut self, values : HashMap<WNKey, Synset>) -> Result<(),WordNetLoadError> {
        {
            let tx = self.conn.transaction()?;
            for (k, v) in values.iter() {
                WordNetBuilder::insert_synset(&tx, k.clone(), v.clone())?;
            }
            tx.commit()?;
        }
        for (k, v) in values {
            self.insert_synset2(k, v);
        }
        Ok(())
    }

    pub fn recommit_synsets(&mut self) -> Result<(),WordNetLoadError> {
        let tx = self.conn.transaction()?;
        for (k, v) in self.synsets.iter() {
            WordNetBuilder::recommit_synset(&tx, k.clone(), v.clone())?;
        }
        tx.commit()?;
        Ok(())
    }
    
    pub fn update_synset(&mut self, key : WNKey, synset : Synset) -> Result<(), WordNetLoadError> {
    //    let key_str = key.to_string();
//        let val_str = serde_json::to_string(&synset)?;
  //      self.conn.execute("UPDATE synsets SET json=? WHERE key=?",
//                     &[&key_str, &val_str])?;
        self.synsets.insert(key, synset);
        Ok(())
    }

    fn insert_synset(tx : &rusqlite::Transaction,
                     key : WNKey, synset : Synset) 
          -> Result<(),WordNetLoadError> {
        let key_str = key.to_string();
        let val_str = serde_json::to_string(&synset)?;
        tx.execute("INSERT INTO synsets (key, ili, json) 
                      VALUES (?1, ?2, ?3)",
                     &[&key_str, &synset.ili, &val_str])?;
        for sense in synset.lemmas {
            tx.execute("INSERT INTO lemmas (lemma, form, language, synset) VALUES (?,?,?,?)", 
                         &[&sense.lemma, &sense.lemma.to_lowercase(), &sense.language, &key_str])?;
            for form in sense.forms {
                tx.execute("INSERT INTO lemmas (lemma, form, language, synset) VALUES (?,?,?,?)",
                        &[&sense.lemma, &form.to_lowercase(), &sense.language, &key_str])?;
            }
            match sense.sense_key {
                Some(ref sense_key) => {
                    tx.execute("INSERT INTO sense_keys (sense_key, synset)
                                  VALUES (?1, ?2)",
                                 &[sense_key, &key_str])?;
                },
                None => {}
            }
        }
        Ok(())
    }

    fn recommit_synset(tx : &rusqlite::Transaction,
                       key : WNKey, synset : Synset) 
            -> Result<(),WordNetLoadError> {
        let key_str = key.to_string();
        let val_str = serde_json::to_string(&synset)?;
        tx.execute("UPDATE synsets SET json=? WHERE key=?", 
                     &[&val_str, &key_str])?;
        Ok(())
    }


    fn insert_synset2(&mut self, key : WNKey, synset : Synset) {
        self.synsets.insert(key.clone(), synset.clone());
        self.by_ili.insert(synset.ili.clone(), key.clone());
        for sense in synset.lemmas {
            match sense.sense_key {
                Some(ref sense_key) => {
                    self.by_sense_key.insert(sense_key.clone(), key.clone());
                },
                None => {}
            }
        }
    }

    /// Add a link set to the database
    pub fn insert_links(&mut self, link_type : LinkType,
                        values : Vec<(WNKey, String)>) -> Result<(), WordNetLoadError> {
        for &(ref key, ref target) in values.iter() {
            if let Some(synset) = self.synsets.get_mut(key) {
                synset.links.push(Link {
                    link_type: link_type.clone(),
                    target: target.clone()
                })
            }
        }
        let tx = self.conn.transaction()?;
        for (key, target) in values {
            tx.execute("INSERT INTO links VALUES (?1, ?2, ?3)",
                     &[&key.to_string(), 
                       &serde_json::to_string(&link_type)?, 
                       &target])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Add a set of old links to the database
    pub fn set_old_ids(&mut self, index : &str, values : Vec<(WNKey, WNKey)>)
        -> Result<(),WordNetLoadError> {
         for &(ref old, ref new) in values.iter() {
             if let Some(synset) = self.synsets.get_mut(new) {
                 synset.old_keys.entry(index.to_owned())
                     .or_insert_with(|| Vec::new())
                     .push(old.clone());
             }
         }
         if index == "pwn30" {
             for &(ref k, ref v) in values.iter() {
                 self.by_pwn30.insert(k.clone(), v.clone());
             }
         }
         if index == "pwn20" {
             for &(ref k, ref v) in values.iter() {
                 self.by_pwn20.insert(k.clone(), v.clone());
             }
         }
         let tx = self.conn.transaction()?;
         for (old_id, id) in values {
             tx.execute("INSERT INTO old_keys
                       VALUES (?, ?, ?)",
                       &[&index.to_string(), &old_id.to_string(), 
                       &id.to_string()])?;
         }
         tx.commit()?;
         Ok(())
    }

    pub fn get_id_by_ili(&self, ili : &str) -> 
        Result<Option<WNKey>,WordNetLoadError> {
        Ok(self.by_ili.get(ili).map(|x| x.clone()))
    }

    pub fn get_id_by_sense_key(&self, sense_key : &str) -> 
        Result<Option<WNKey>,WordNetLoadError> {
        Ok(self.by_sense_key.get(sense_key).map(|x| x.clone()))
    }

    pub fn get_synset(&self, id : &WNKey) ->
        Result<Option<Synset>,WordNetLoadError> {
        Ok(self.synsets.get(id).map(|x| x.clone()))
    }

    pub fn get_id_by_pwn30(&self, key : &WNKey) ->
        Result<Option<WNKey>,WordNetLoadError> {
        Ok(self.by_pwn30.get(key).map(|x| x.clone()))
    }

    pub fn get_id_by_pwn20(&self, key : &WNKey) ->
        Result<Option<WNKey>,WordNetLoadError> {
        Ok(self.by_pwn20.get(key).map(|x| x.clone()))
    }

    pub fn finalize(&mut self) -> Result<WordNet,WordNetLoadError> { 
        self.recommit_synsets()?;
        Ok(WordNet)
    }
}

fn ok_wnkey(s : String) -> Result<String, ::std::io::Error> {
    Ok(s)
}

impl WordNet {
    fn open_conn() -> Result<rusqlite::Connection,WordNetLoadError> {
        rusqlite::Connection::open("wordnet.db")            
            .map_err(|e| WordNetLoadError::SQLite(e))
    }

    pub fn new() -> WordNet { WordNet { } }
 
    #[allow(dead_code)]
    pub fn get_synset_ids(&self) -> Result<Vec<WNKey>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT key FROM synsets",
                         &[], ok_wnkey)
    }

    pub fn get_synset(&self, key : &WNKey) -> Result<Option<Synset>,WordNetLoadError> { 
        sqlite_query_opt_map("SELECT json FROM synsets WHERE key=?",
                             &[&key.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
    pub fn get_by_lemma(&self, lemma : &str, lang : &str) -> Result<Vec<Synset>,WordNetLoadError> { 
        sqlite_query_vec("SELECT DISTINCT json FROM synsets
                          JOIN lemmas ON lemmas.synset=synsets.key
                          WHERE lemma=? AND language=?",
                          &[&lemma.to_owned(), &lang.to_owned()],
                          |s| { serde_json::from_str(&s) })
    }
//    pub fn get_id_by_ili(&self, ili : &str) -> Result<Option<WNKey>,WordNetLoadError> {
//        sqlite_query_opt_map("SELECT key FROM synsets WHERE ili=?",
//                             &[&ili.to_string()],
//                             |s| { WNKey::from_str(&s) })
//    }
    pub fn get_by_ili(&self, ili : &str) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets WHERE ili=?",
                             &[&ili.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
//    pub fn get_id_by_sense_key(&self, sense_key : &str) -> Result<Option<WNKey>,WordNetLoadError> {
//        sqlite_query_opt_map("SELECT synset FROM sense_keys WHERE sense_key=?",
//                             &[&sense_key.to_string()],
//                             |s| { WNKey::from_str(&s) })
//    }
    pub fn get_by_sense_key(&self, sense_key : &str) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets
                              JOIN sense_keys ON sense_keys.synse=synsets.key
                              WHERE sense_key=?",
                             &[&sense_key.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
//    pub fn get_id_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<WNKey>,WordNetLoadError> {
//        sqlite_query_opt_map("SELECT synset FROM old_keys
//                              WHERE key=? AND idx=?",
//                             &[&id.to_string(), &index.to_string()],
//                             |s| { WNKey::from_str(&s) })
//    }
    pub fn get_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets
                              JOIN old_keys ON old_keys.synset=synsets.key
                              WHERE old_keys.key=? AND idx=?",
                             &[&id.to_string(), &index.to_string()],
                             |s| { serde_json::from_str(&s) })
    }

    pub fn list_by_id(&self, key : &WNKey, 
                      limit : u32) -> Result<Vec<WNKey>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT key FROM synsets
                          WHERE key >= ?
                          ORDER BY key
                          LIMIT ?",
                         &[&key.to_string(), &limit],ok_wnkey)
                         // |s| { WNKey::from_str(&s) })
    }
    pub fn list_by_lemma(&self, lemma : &String, language : &str,
                          limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT lemma FROM lemmas
                          WHERE form >= ? and form like ? and language=?
                          ORDER BY form
                          LIMIT ?",
                         &[&lemma.to_lowercase(), 
                            &(lemma.to_lowercase() + "%"),
                            &language.to_string(),
                         &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_ili(&self, ili : &String,
                        limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT ili FROM synsets
                          WHERE ili >= ?
                          ORDER BY ili
                          LIMIT ?",
                         &[&ili.to_string(), &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_sense_key(&self, sense_key : &String,
                              limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT sense_key FROM sense_keys
                          WHERE sense_key >= ?
                          ORDER BY sense_key
                          LIMIT ?",
                         &[&sense_key.to_string(), &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_old_id(&self, index : &str, key : &WNKey,
                      limit : u32) -> Result<Vec<WNKey>,WordNetLoadError> {
        sqlite_query_vec("SELECT DISTINCT key FROM old_keys
                          WHERE key >= ? AND idx=?
                          ORDER BY key
                          LIMIT ?",
                         &[&key.to_string(), &index.to_string(), &limit], 
                         ok_wnkey)// { WNKey::from_str(&s) })
    }

}

quick_error! {
    #[derive(Debug)]
    pub enum WordNetLoadError {
        Io(err: ::std::io::Error) { 
            from()
            display("I/O error: {}", err)
            cause(err)
        }
        Xml(err: ::xml::reader::Error) {
            from()
            display("XML error: {}", err)
            cause(err)
        }
        Schema(msg : &'static str) {
            description(msg)
        }
        JsonSerialization(err : ::serde_json::Error) {
            from()
            display("JSON error: {}", err)
            cause(err)
        }
        SQLite(err : rusqlite::Error) {
            from()
            display("SQLite error: {}", err)
            cause(err)
        }
//        BadKey(msg : String) {
//            description(msg)
//        }
    }
}
