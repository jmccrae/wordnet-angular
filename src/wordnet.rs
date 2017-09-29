//! Functions for handling the in-memory model of WordNet and loading it form
//! disk
use glosstag::{Gloss,read_glosstag_corpus};
use omwn::load_omwn;
use serde::de::{Visitor, Deserializer, Error as DeError};
use serde::{Serialize, Serializer,Deserialize};
use stable_skiplist::OrderedSkipList;
use std::collections::HashMap;
use std::fmt::{Formatter, Result as FormatResult};
use std::fs::File;
use std::io::{BufRead,BufReader};
use std::path::Path;
use std::str::FromStr;
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};
use links::{Link,load_links};

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

//impl WNKey {
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
//}

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

pub struct WordNet {
    pub synsets : HashMap<WNKey, Synset>,
    pub by_lemma : HashMap<String, Vec<WNKey>>,
    pub by_ili : HashMap<String, WNKey>,
    pub by_sense_key : HashMap<String, WNKey>,
    pub by_old_id : HashMap<String, HashMap<WNKey, WNKey>>,
    pub id_skiplist : OrderedSkipList<WNKey>,
    pub lemma_skiplist : OrderedSkipList<String>,
    pub ili_skiplist : OrderedSkipList<String>,
    pub sense_key_skiplist : OrderedSkipList<String>,
    pub old_skiplist : HashMap<String, OrderedSkipList<WNKey>>
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
        let mut wordnet = WordNet{
            synsets: synsets,
            by_lemma: HashMap::new(),
            by_ili: HashMap::new(),
            by_sense_key: HashMap::new(),
            by_old_id: HashMap::new(),
            lemma_skiplist: lemma_skiplist,
            ili_skiplist: OrderedSkipList::new(),
            id_skiplist: OrderedSkipList::new(),
            sense_key_skiplist: OrderedSkipList::new(),
            old_skiplist: HashMap::new()
        };
        build_indexes(&mut wordnet);
        build_tabs(&mut wordnet)?;
        //build_glosstags(&mut wordnet)?;
        build_omwn(&mut wordnet)?;
        load_links(&mut wordnet);
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
    let ref wn30_idx = wordnet.by_old_id["pwn30"];
    let ref sense_keys = wordnet.by_sense_key;
    eprintln!("Loading gloss tags (adj)");
    let mut result = read_glosstag_corpus("data/merged/adj.xml", wn30_idx, sense_keys)?;
    eprintln!("Loading gloss tags (adv)");
    result.extend(read_glosstag_corpus("data/merged/adv.xml", wn30_idx, sense_keys)?);
    eprintln!("Loading gloss tags (noun)");
    result.extend(read_glosstag_corpus("data/merged/noun.xml", wn30_idx, sense_keys)?);
    eprintln!("Loading gloss tags (verb)");
    result.extend(read_glosstag_corpus("data/merged/verb.xml", wn30_idx, sense_keys)?);
    for (k,v) in result.iter() {
        wordnet.synsets.get_mut(k)
            .map(|x| {
                x.gloss = Some(v.clone())
            });
    }
    Ok(())
}

fn build_tab<P : AsRef<Path>>(file : P, 
    index : &str,
    by_ili : &HashMap<String, WNKey>,
    synsets : &mut HashMap<WNKey, Synset>,
    by_tab : &mut HashMap<WNKey, WNKey>,
    skiplist : &mut OrderedSkipList<WNKey>) -> Result<(),WordNetLoadError> {
    let file = BufReader::new(File::open(file)?);
    for line in file.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        match elems.next() {
            Some(ili) => {
                match elems.next() {
                    Some(id) => {
                        skiplist.insert(WNKey::from_str(id)?);
                        match by_ili.get(ili) {
                            Some(wnid) => {
                                by_tab.insert(WNKey::from_str(id)?, wnid.clone());
                                synsets.entry(wnid.clone())
                                    .or_insert_with(|| panic!("ILI not in WordNet??"))
                                    .old_keys.entry(index.to_string())
                                    .or_insert_with(|| Vec::new())
                                    .push(WNKey::from_str(id)?);
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
    

fn build_indexes(wordnet : &mut WordNet) {
    eprintln!("Building indexes");
    for (id, synset) in wordnet.synsets.iter() {
        wordnet.ili_skiplist.insert(synset.ili.clone());
        wordnet.id_skiplist.insert(id.clone());
        wordnet.by_ili.insert(synset.ili.clone(), id.clone());
        for sense in synset.lemmas.iter() {
            wordnet.by_sense_key.insert(sense.sense_key.clone(), id.clone());
            wordnet.sense_key_skiplist.insert(sense.sense_key.clone());
            wordnet.by_lemma.entry(sense.lemma.clone())
                .or_insert_with(|| Vec::new())
                .push(id.clone());
        }
    }
}

fn build_tabs(wordnet : &mut WordNet) -> Result<(),WordNetLoadError> {
    for tab in ["pwn15", "pwn16", "pwn17", "pwn171", "pwn20",
                "pwn21", "pwn30"].iter() {
        eprintln!("Loading Tab {}", tab);
        let mut map = HashMap::new();
        let mut list = OrderedSkipList::new();
        let path = format!("data/ili-map-{}.tab", tab);
        build_tab(path, tab, &wordnet.by_ili, 
            &mut wordnet.synsets, &mut map, &mut list)?;
        wordnet.by_old_id.insert(tab.to_string(), map);
        wordnet.old_skiplist.insert(tab.to_string(), list);
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
            &wordnet.by_old_id["pwn30"])?;
       for (key, values) in omwn.iter() {
           match wordnet.synsets.get_mut(&key) {
               Some(s) => {
                   let mut vs = values.clone();
                   vs.dedup();
                   s.foreign.insert(lang.to_string(), vs);
               },
               None => {}
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
        BadKey(msg : String) {
            description(msg)
        }
    }
}
