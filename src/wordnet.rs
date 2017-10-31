//! Functions for handling the in-memory model of WordNet and loading it form
//! disk
use glosstag::{Gloss,read_glosstag_corpus};
use omwn::load_omwn;
use serde::de::{Visitor, Deserializer, Error as DeError};
use serde::{Serialize, Serializer,Deserialize};
use stable_skiplist::OrderedSkipList;
use std::collections::HashMap;
use std::fmt::{Formatter, Result as FormatResult};
use std::fs::{File};
use std::io::{BufRead,BufReader};
use std::path::Path;
use std::str::FromStr;
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};
use links::{Link,load_links,LinkType};
use serde_json;
use rusqlite;

/// A WordNet part of speech
#[derive(Clone,Debug)]
pub enum PartOfSpeech {
    Noun, Verb, Adjective, Adverb, AdjectiveSatellite, Other
}

impl FromStr for PartOfSpeech {
    type Err = WordNetLoadError;
    fn from_str(s : &str) -> Result<PartOfSpeech, WordNetLoadError> { 
        match s {
            "n" => Ok(PartOfSpeech::Noun),
            "v" => Ok(PartOfSpeech::Verb),
            "a" => Ok(PartOfSpeech::Adjective),
            "s" => Ok(PartOfSpeech::AdjectiveSatellite),
            "r" => Ok(PartOfSpeech::Adverb),
            "x" => Ok(PartOfSpeech::Other),
            _ => Err(WordNetLoadError::Schema("Bad part of speech value"))
        }
    }
}

impl ToString for PartOfSpeech {
    fn to_string(&self) -> String {
        match *self {
            PartOfSpeech::Noun => "n".to_string(),
            PartOfSpeech::Verb => "v".to_string(),
            PartOfSpeech::Adjective => "a".to_string(),
            PartOfSpeech::AdjectiveSatellite => "s".to_string(),
            PartOfSpeech::Adverb => "r".to_string(),
            PartOfSpeech::Other => "x".to_string()
        }
    }
}

impl PartOfSpeech {
    pub fn as_long_string(&self) -> &'static str {
        match *self {
            PartOfSpeech::Noun => "noun",
            PartOfSpeech::Verb => "verb",
            PartOfSpeech::Adjective => "adjective",
            PartOfSpeech::AdjectiveSatellite => "adjective_satellite",
            PartOfSpeech::Adverb => "adverb",
            PartOfSpeech::Other => "other"
        }
    }
}

impl Serialize for PartOfSpeech {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PartOfSpeech {
    fn deserialize<D>(deserializer: D) -> Result<PartOfSpeech, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_str(PartOfSpeechVisitor)        
    }
}

struct PartOfSpeechVisitor;

impl<'de> Visitor<'de> for PartOfSpeechVisitor {
    type Value = PartOfSpeech;

    fn expecting(&self, formatter : &mut Formatter) -> FormatResult {
        formatter.write_str("A part of speech value as a single letter: n,v,a,r,s or x")
    }

    fn visit_str<E>(self, value : &str) -> Result<PartOfSpeech, E>  where E : DeError {
        PartOfSpeech::from_str(value)
            .map_err(|e| E::custom(e))
    }

    fn visit_string<E>(self, value : String) -> Result<PartOfSpeech, E> where E : DeError {
        PartOfSpeech::from_str(&value)
            .map_err(|e| E::custom(e))
    }
}




/// A WordNet Key consisting of 8 digits and a part of speech.
/// This data structure stores the value as a 4-byte integer to save memory
#[derive(Clone,Debug,PartialEq,Eq,Hash,PartialOrd)]
pub struct WNKey(u32);

impl WNKey {
//    /// Create from an ID and a part of speech
//    pub fn new(id : u32, pos : char) -> Result<WNKey, WordNetLoadError> {
//        match pos {
//            'n' => Ok(WNKey((id << 8) + 1)),
//            'v' => Ok(WNKey((id << 8) + 2)),
//            'a' => Ok(WNKey((id << 8) + 3)),
//            'r' => Ok(WNKey((id << 8) + 4)),
//            's' => Ok(WNKey((id << 8) + 5)),
//            'p' => Ok(WNKey((id << 8) + 6)),
//            'x' => Ok(WNKey((id << 8) + 7)),
//            _ => Err(WordNetLoadError::BadKey(format!("Bad WN POS: {}", pos)))
//        }
//    }
//        
    pub fn from_slice(slice : &[u8]) -> Result<WNKey, WordNetLoadError> {
        let s = String::from_utf8(Vec::from(slice))
            .map_err(|_| WordNetLoadError::BadKey(format!("Invalid UTF-8")))?;
        WNKey::from_str(&s)
    }
}

impl FromStr  for WNKey {
    type Err = WordNetLoadError;
    fn from_str(s : &str) -> Result<WNKey, WordNetLoadError> { 
        if s.len() != 10 {
            Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
        } else {
            let num = u32::from_str(&s.chars().take(8).collect::<String>())
                .map_err(|_| WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))? << 8;
            match s.chars().skip(9).next() {
                Some('n') => Ok(WNKey(0x00000001 | num)),
                Some('v') => Ok(WNKey(0x00000002 | num)),
                Some('a') => Ok(WNKey(0x00000003 | num)),
                Some('r') => Ok(WNKey(0x00000004 | num)),
                Some('s') => Ok(WNKey(0x00000005 | num)),
                Some('p') => Ok(WNKey(0x00000006 | num)),
                Some('x') => Ok(WNKey(0x00000007 | num)),
                _ => Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
            }
        }
    }
}

impl ToString for WNKey {
    fn to_string(&self) -> String { 
        match self.0 & 0x0000000f {
            1 => format!("{:08}-n", (self.0 & 0xfffffff0) >> 8),
            2 => format!("{:08}-v", (self.0 & 0xfffffff0) >> 8),
            3 => format!("{:08}-a", (self.0 & 0xfffffff0) >> 8),
            4 => format!("{:08}-r", (self.0 & 0xfffffff0) >> 8),
            5 => format!("{:08}-s", (self.0 & 0xfffffff0) >> 8),
            6 => format!("{:08}-p", (self.0 & 0xfffffff0) >> 8),
            7 => format!("{:08}-x", (self.0 & 0xfffffff0) >> 8),
            _ => format!("{:08}-?", (self.0 & 0xfffffff0) >> 8)
        }
    }
}

impl Serialize for WNKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WNKey {
    fn deserialize<D>(deserializer: D) -> Result<WNKey, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_str(WNKeyVisitor)        
    }
}

struct WNKeyVisitor;

impl<'de> Visitor<'de> for WNKeyVisitor {
    type Value = WNKey;

    fn expecting(&self, formatter : &mut Formatter) -> FormatResult {
        formatter.write_str("A WordNet key such as 00001740-a")
    }

    fn visit_str<E>(self, value : &str) -> Result<WNKey, E>  where E : DeError {
        WNKey::from_str(value)
            .map_err(|e| E::custom(e))
    }

    fn visit_string<E>(self, value : String) -> Result<WNKey, E> where E : DeError {
        WNKey::from_str(&value)
            .map_err(|e| E::custom(e))
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Synset {
    pub definition : String,
    pub lemmas : Vec<Sense>,
    pub id : WNKey,
    pub ili : String,
    pub pos : PartOfSpeech,
    pub subject : String,
    pub relations : Vec<Relation>,
    pub old_keys : HashMap<String, Vec<WNKey>>,
    pub gloss : Option<Vec<Gloss>>,
    pub foreign : HashMap<String, Vec<String>>,
    pub links : Vec<Link>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Sense {
    pub lemma : String,
    pub forms : Vec<String>,
    pub sense_key : String,
    pub subcats : Vec<String>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Relation {
    pub src_word : Option<String>,
    pub trg_word : Option<String>,
    pub rel_type : String,
    pub target : String
}

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

pub struct WordNet { }

fn ok_wordnet_str(s : String) -> Result<String, WordNetLoadError> {
    Ok(s)
}

impl WordNet {
    fn open_conn() -> Result<rusqlite::Connection,WordNetLoadError> {
        rusqlite::Connection::open("wordnet.db")            
            .map_err(|e| WordNetLoadError::SQLite(e))
    }

    pub fn new() -> Result<WordNet,WordNetLoadError> {
        let conn = WordNet::open_conn()?;
        conn.execute("CREATE TABLE synsets (
                      key TEXT NOT NULL,
                      ili TEXT NOT NULL,
                      json TEXT NOT NULL)", &[])?;
        conn.execute("CREATE INDEX synsets_key ON synsets (key)", &[])?;
        conn.execute("CREATE INDEX synsets_ili ON synsets (ili)", &[])?;
        conn.execute("CREATE TABLE lemmas (
                      lemma TEXT NOT NULL,
                      synset TEXT NOT NULL,
                      FOREIGN KEY (synset) REFERENCES synsets (key))", &[])?;
        conn.execute("CREATE INDEX lemmas_lemma ON lemmas (lemma)", &[])?;
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
        Ok(WordNet { })
    }

    pub fn new_using_indexes() -> WordNet { WordNet { } }
 
    pub fn set_synsets(&mut self, values : HashMap<WNKey, Synset>) -> Result<(),WordNetLoadError> {
        for (k, v) in values {
            self.insert_synset(k, v)?;
        }
        Ok(())
    }
    
    /// Only update the synset's JSON file, does not change any quierable 
    /// properties
    pub fn update_synset(&mut self, key : WNKey, synset : Synset) -> Result<(), WordNetLoadError> {
        let key_str = key.to_string();
        let val_str = serde_json::to_string(&synset)?;
        let conn = WordNet::open_conn()?;
        conn.execute("UPDATE synsets SET json=? WHERE key=?",
                     &[&key_str, &val_str])?;
        Ok(())
    }

    pub fn insert_synset(&mut self, key : WNKey, synset : Synset) -> Result<(),WordNetLoadError> {
        let key_str = key.to_string();
        let val_str = serde_json::to_string(&synset)?;
        let conn = WordNet::open_conn()?;
        conn.execute("INSERT INTO synsets (key, ili, json) 
                      VALUES (?1, ?2, ?3)",
                     &[&key_str, &synset.ili, &val_str])?;
        for sense in synset.lemmas {
            conn.execute("INSERT INTO lemmas (lemma, synset)
                          VALUES (?1, ?2)", 
                         &[&sense.lemma, &key_str])?;
            conn.execute("INSERT INTO sense_keys (sense_key, synset)
                          VALUES (?1, ?2)",
                         &[&sense.sense_key, &key_str])?;
        }
        Ok(())
    }

    /// Add a link to the database
    pub fn insert_link(&self, key : &WNKey, link_type : LinkType, 
                       target : String) -> Result<(), WordNetLoadError> {
        let conn = WordNet::open_conn()?;
        conn.execute("INSERT INTO links VALUES (?1, ?2, ?3)",
                     &[&key.to_string(), 
                       &serde_json::to_string(&link_type)?, 
                       &target])?;
        Ok(())
    }
    

    pub fn get_synset(&self, key : &WNKey) -> Result<Option<Synset>,WordNetLoadError> { 
        sqlite_query_opt_map("SELECT json FROM synsets WHERE key=?",
                             &[&key.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
    pub fn get_by_lemma(&self, lemma : &str) -> Result<Vec<Synset>,WordNetLoadError> { 
        sqlite_query_vec("SELECT json FROM synsets
                          JOIN lemmas ON lemmas.synset=synsets.key
                          WHERE lemma=?",
                          &[&lemma.to_owned()],
                          |s| { serde_json::from_str(&s) })
    }
    pub fn get_id_by_ili(&self, ili : &str) -> Result<Option<WNKey>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT key FROM synsets WHERE ili=?",
                             &[&ili.to_string()],
                             |s| { WNKey::from_str(&s) })
    }
    pub fn get_by_ili(&self, ili : &str) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets WHERE ili=?",
                             &[&ili.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
    pub fn get_id_by_sense_key(&self, sense_key : &str) -> Result<Option<WNKey>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT synset FROM sense_keys WHERE sense_key=?",
                             &[&sense_key.to_string()],
                             |s| { WNKey::from_str(&s) })
    }
    pub fn get_by_sense_key(&self, sense_key : &str) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets
                              JOIN sense_keys ON sense_keys.synse=synsets.key
                              WHERE sense_key=?",
                             &[&sense_key.to_string()],
                             |s| { serde_json::from_str(&s) })
    }
    pub fn get_id_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<WNKey>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT synset FROM old_keys
                              WHERE key=? AND idx=?",
                             &[&id.to_string(), &index.to_string()],
                             |s| { WNKey::from_str(&s) })
    }
    pub fn get_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<Synset>,WordNetLoadError> {
        sqlite_query_opt_map("SELECT json FROM synsets
                              JOIN old_keys ON old_keys.synset=synsets.key
                              WHERE key=? AND idx=?",
                             &[&id.to_string(), &index.to_string()],
                             |s| { serde_json::from_str(&s) })
    }

     pub fn set_old_id(&mut self, index : &str, id : &WNKey, old_id : &WNKey) -> Result<(),WordNetLoadError> {
         let conn = WordNet::open_conn()?;
         conn.execute("INSERT INTO old_keys
                       VALUES (?, ?, ?)",
                      &[&index.to_string(), &old_id.to_string(), 
                        &id.to_string()])?;
         Ok(())
    }
    pub fn list_by_id(&self, key : &WNKey, 
                      limit : u32) -> Result<Vec<WNKey>,WordNetLoadError> {
        sqlite_query_vec("SELECT key FROM synsets
                          WHERE key >= ?
                          ORDER BY key
                          LIMIT ?",
                         &[&key.to_string(), &limit],
                         |s| { WNKey::from_str(&s) })
    }
    pub fn list_by_lemma(&self, lemma : &String,
                          limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT lemma FROM lemmas
                          WHERE lemma >= ?
                          ORDER BY lemma
                          LIMIT ?",
                         &[&lemma.to_string(), &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_ili(&self, ili : &String,
                        limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT ili FROM synsets
                          WHERE ili >= ?
                          ORDER BY ili
                          LIMIT ?",
                         &[&ili.to_string(), &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_sense_key(&self, sense_key : &String,
                              limit : u32) -> Result<Vec<String>,WordNetLoadError> {
        sqlite_query_vec("SELECT sense_key FROM sense_keys
                          WHERE sense_key >= ?
                          ORDER BY sense_key
                          LIMIT ?",
                         &[&sense_key.to_string(), &limit], 
                         ok_wordnet_str)
    }
    pub fn list_by_old_id(&self, index : &str, key : &WNKey,
                      limit : u32) -> Result<Vec<WNKey>,WordNetLoadError> {
        sqlite_query_vec("SELECT key FROM old_keys
                          WHERE key >= ? AND idx=?
                          ORDER BY key
                          LIMIT ?",
                         &[&key.to_string(), &index.to_string(), &limit], 
                         |s| { WNKey::from_str(&s) })
    }
}

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

fn clean_id(s : &str) -> Result<WNKey, WordNetLoadError> {
    let s2 : String = s.chars().skip(5).collect();
    WNKey::from_str(&s2)
}

impl WordNet {
    pub fn load<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
        let file = BufReader::new(File::open(path)?);

        let parse = EventReader::new(file);

        let mut lexical_entry_id : Option<String> = None;
        let mut entry_lemma = None;
        let mut sense_keys = HashMap::new();
        let mut entry_id_to_lemma = HashMap::new();
        let mut synset_to_entry = HashMap::new();
        let mut sense_to_lemma = HashMap::new();
        let mut sense_to_synset = HashMap::new();
        let mut subcats = HashMap::new();
        let mut synset = None;
        let mut synset_id = None;
        let mut synset_ili_pos_subject = None;
        let mut in_def = false;
        let mut definition = None;
        let mut synsets = HashMap::new();
        let mut relations : HashMap<WNKey,Vec<Relation>> = HashMap::new();

        let mut lemma_skiplist = OrderedSkipList::new();

        for e in parse {
            match e {
                Ok(XmlEvent::StartElement{ name, attributes, .. }) => {
                    if name.local_name == "LexicalEntry" {
                        match attr_value(&attributes, "id") {
                            Some(id) => {
                                lexical_entry_id = Some(id)
                            },
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "LexicalEntry does not have an ID"));
                            }
                        }
                    } else if name.local_name == "Lemma" {
                        let entry_id = match lexical_entry_id {
                            Some(ref i) => i.clone(),
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma outside of LexicalEntry"))
                            }
                        };
                        let lemma = match attr_value(&attributes, "writtenForm") {
                            Some(l) => l,
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma does not have writtenForm"));
                            }
                        };
                        entry_lemma = Some(lemma.clone());
                        entry_id_to_lemma.insert(entry_id, lemma.clone());
                        if !lemma_skiplist.contains(&lemma) {
                            lemma_skiplist.insert(lemma);
                        }
                    } else if name.local_name == "Sense" {
                        let entry_id = match lexical_entry_id {
                            Some(ref i) => i.clone(),
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma outside of LexicalEntry"))
                            }
                         };
                         let target = clean_id(&attr_value(&attributes, "synset")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                    "Sense does not have a synset"))?)?;
                         match attr_value(&attributes, "identifier") {
                            Some(i) => {
                                sense_keys.entry(entry_id.clone())
                                    .or_insert_with(|| HashMap::new())
                                    .insert(target.clone(), i);
                            },
                            None => {}
                         };
                         synset = Some(target.clone());
                         synset_to_entry.entry(target.clone())
                            .or_insert_with(|| Vec::new())
                            .push(entry_id);
                         let sense_id = attr_value(&attributes, "id")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Sense without id"))?;
                         let word = entry_lemma.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation before Lemma"))?;
                         sense_to_lemma.insert(sense_id.clone(), word);
                         sense_to_synset.insert(sense_id, target);
                    } else if name.local_name == "SenseRelation" {
                        let typ = attr_value(&attributes, "relType")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation without relType"))?;
                        let targ = attr_value(&attributes, "target")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation without target"))?;
                        let ss = synset.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation outside of Sense"))?;
                        let word = entry_lemma.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation before Lemma"))?;
                        relations.entry(ss)
                            .or_insert_with(|| Vec::new())
                            .push(Relation {
                                src_word: Some(word),
                                trg_word: None,
                                rel_type: typ,
                                target: targ
                            });
                    } else if name.local_name == "SyntacticBehaviour" {
                        let entry_id = lexical_entry_id.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SyntacticBehaviour outside of LexicalEntry"))?;
                        let subcat = attr_value(&attributes, "subcategorizationFrame")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SyntacticBehaviour has no subcategorizationFrame"))?;
                        subcats.entry(entry_id)
                            .or_insert_with(|| Vec::new())
                            .push(subcat);
                    } else if name.local_name == "Synset" {
                        synset_id = Some(clean_id(&attr_value(&attributes, "id")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                    "Synset does not have an id"))?)?);
                        synset_ili_pos_subject = Some((
                            attr_value(&attributes, "ili")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?,
                            attr_value(&attributes, "partOfSpeech")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?,
                            attr_value(&attributes, "subject")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?));
                    } else if name.local_name == "Definition" {
                        in_def = true;
                    } else if name.local_name == "SynsetRelation" {
                        let typ = attr_value(&attributes, "relType")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation without relType"))?;
                        let targ = clean_id(&attr_value(&attributes, "target")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation without target"))?)?;
                        let ss = synset_id.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation outside of Sense"))?;
                        relations.entry(ss)
                            .or_insert_with(|| Vec::new())
                            .push(Relation {
                                src_word: None,
                                trg_word: None,
                                rel_type: typ,
                                target: targ.to_string()
                            });
    }
                },
                Ok(XmlEvent::EndElement { name, .. }) => {
                    if name.local_name == "LexicalEntry" {
                        lexical_entry_id = None;
                        entry_lemma = None;
                    } else if name.local_name == "Sense" {
                        synset = None;
                    } else if name.local_name == "Synset" {
                        let defn = definition.ok_or(
                            WordNetLoadError::Schema(
                                "Synset without definition"))?;
                        let ssid = synset_id.unwrap();
                        let entries = synset_to_entry.get(&ssid)
                            .map(|x| x.clone())
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|x| {
                                Sense {
                                    lemma: entry_id_to_lemma.get(x)
                                        .expect("Entry must have lemma")
                                        .clone(),
                                    // TODO
                                    forms: Vec::new(),
                                    sense_key: sense_keys[x][&ssid].clone(),
                                    subcats: subcats.get(x)
                                        .map(|x| x.clone())
                                        .unwrap_or_else(|| Vec::new())
                                        .clone()
                                }
                            })
                            .collect();
                        let (ili, pos, subject) = synset_ili_pos_subject.clone()
                            .expect("ILI/POS/Subject not set");
                        let rels = relations.get(&ssid)
                            .map(|x| x.clone())
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|r| {
                                if r.src_word.is_some() {
                                    Relation {
                                        src_word: r.src_word.clone(),
                                        trg_word: Some(sense_to_lemma[&r.target].clone()),
                                        rel_type: r.rel_type.clone(),
                                        target: sense_to_synset[&r.target].to_string()
                                    }
                                } else {
                                    r.clone()
                                }
                            })
                            .collect();
                        synsets.insert(ssid.clone(),
                            Synset {
                                definition: defn,
                                lemmas: entries,
                                id: ssid,
                                ili: ili,
                                pos: PartOfSpeech::from_str(&pos)?,
                                subject: subject,
                                relations: rels,
                                old_keys: HashMap::new(),
                                gloss: None,
                                foreign: HashMap::new(),
                                links: Vec::new()
                            });
                            
                        synset_id = None;
                        definition = None;
                    } else if name.local_name == "Definition" {
                        in_def = false;
                    }
                },
                Ok(XmlEvent::Characters(s)) => {
                    if in_def {
                        definition = Some(s);
                    }
                },
                Ok(_) => {},
                Err(e) => { return Err(WordNetLoadError::Xml(e)); }
            }
        }
        let mut wordnet = WordNet::new()?;
        wordnet.set_synsets(synsets)?;
        build_tabs(&mut wordnet)?;
        build_glosstags(&mut wordnet)?;
        build_omwn(&mut wordnet)?;
        load_links(&mut wordnet)?;
        //eprintln!("size_of synsets: {}", wordnet.synsets.len());
        //eprintln!("size_of by_lemma: {}", wordnet. by_lemma.len());
        //eprintln!("size_of by_ili: {}", wordnet.by_ili.len());
        //eprintln!("size_of by_sense_key: {}", wordnet.by_sense_key.len());
        //eprintln!("size_of by_old_id: {}", wordnet.by_old_id.len());
        //eprintln!("size_of id_skiplist: {}", wordnet.id_skiplist.len());
        //eprintln!("size_of lemma_skiplist: {}", wordnet.lemma_skiplist.len());
        //eprintln!("size_of ili_skiplist: {}", wordnet.ili_skiplist.len());
        //eprintln!("size_of sense_key_skiplist: {}", wordnet.sense_key_skiplist.len());
        //eprintln!("size_of old_skiplist: {}", wordnet.old_skiplist.len());

        Ok(wordnet)
    }
}

fn build_glosstags(wordnet : &mut WordNet)
         -> Result<(), WordNetLoadError> {
    eprintln!("Loading gloss tags (adj)");
    let mut result = read_glosstag_corpus("data/merged/adj.xml", &wordnet)?;
    eprintln!("Loading gloss tags (adv)");
    result.extend(read_glosstag_corpus("data/merged/adv.xml", &wordnet)?);
    eprintln!("Loading gloss tags (noun)");
    result.extend(read_glosstag_corpus("data/merged/noun.xml", &wordnet)?);
    eprintln!("Loading gloss tags (verb)");
    result.extend(read_glosstag_corpus("data/merged/verb.xml", &wordnet)?);
    for (k,v) in result.iter() {
        if let Some(mut s) = wordnet.get_synset(k)? {
            s.gloss = Some(v.clone());
            wordnet.update_synset(k.clone(), s)?;
        }
    }
    Ok(())
}

fn build_tab<P : AsRef<Path>>(file : P, 
    index : &str,
    wordnet : &mut WordNet) -> Result<(),WordNetLoadError> {
    let file = BufReader::new(File::open(file)?);
    for line in file.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        match elems.next() {
            Some(ili) => {
                match elems.next() {
                    Some(id) => {
                        let wnid2 = wordnet.get_id_by_ili(ili)?.map(|x| x.clone());
                        match wnid2 {
                            Some(wnid) => {
                                wordnet.set_old_id(index, 
                                    &wnid,
                                    &WNKey::from_str(id)?)?
                            },
                            None => {}
                        };
                    },
                    None => {}
                }
            },
            None => {}
        }
    }
    Ok(())
}
    

//fn build_indexes(wordnet : &mut WordNet) {
//    eprintln!("Building indexes");
//    for (id, synset) in wordnet.synsets.iter() {
//        wordnet.ili_skiplist.insert(synset.ili.clone());
//        wordnet.id_skiplist.insert(id.clone());
//        wordnet.by_ili.insert(synset.ili.clone(), id.clone());
//        for sense in synset.lemmas.iter() {
//            wordnet.by_sense_key.insert(sense.sense_key.clone(), id.clone());
//            wordnet.sense_key_skiplist.insert(sense.sense_key.clone());
//            wordnet.by_lemma.entry(sense.lemma.clone())
//                .or_insert_with(|| Vec::new())
//                .push(id.clone());
//        }
//    }
//}

fn build_tabs(wordnet : &mut WordNet) -> Result<(),WordNetLoadError> {
    for tab in ["pwn15", "pwn16", "pwn17", "pwn171", "pwn20",
                "pwn21", "pwn30"].iter() {
        eprintln!("Loading Tab {}", tab);
        let path = format!("data/ili-map-{}.tab", tab);
        build_tab(path, tab, wordnet)?;
    }
    Ok(())
}

fn build_omwn(wordnet : &mut WordNet) -> Result<(), WordNetLoadError> {
    for lang in ["als","arb","bul","cmn","qcn","ell","fas","fin","fra",
                 "heb","hrv","isl","ita","jpn","cat","eus","glg","spa",
                 "ind","zsm","nld","nno","nob","pol","por","ron",
                 "slk","lit","slv","swe","tha"].iter() {
        eprintln!("Loading OMWN {}", lang);
        let project = match *lang {
            "cmn" => "cow",
            "qcn" => "cwn",
            "ind" => "msa",
            "zsm" => "msa",
            "cat" => "mcr",
            "eus" => "mcr",
            "glg" => "mcr",
            "spa" => "mcr",
            "nno" => "nor",
            "nob" => "nor",
            "lit" => "slk",
            x => x
        };
       let omwn = load_omwn(format!("data/wns/{}/wn-data-{}.tab", project, lang),
            &wordnet)?;
       for (key, values) in omwn.iter() {
           if let Some(mut s2) = wordnet.get_synset(&key)? {
               let mut vs = values.clone();
               vs.dedup();
               s2.foreign.insert(lang.to_string(), vs);
               wordnet.update_synset(key.clone(), s2)?;
           }
       }
    }
    Ok(())

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
        BadKey(msg : String) {
            description(msg)
        }
    }
}
