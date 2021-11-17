//! The model for a wordnet, i.e., the structs for entries, senses and synsets
use serde::de::{Visitor, Deserializer, Error as DeError};
use serde::{Serialize, Serializer,Deserialize};
use std::collections::HashMap;
use wordnet::WordNetLoadError;
use std::fmt::{Formatter, Result as FormatResult};
use std::str::FromStr;
use links::Link;
use glosstag::Gloss;

type WNKey=String;

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

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Synset {
    pub definition : String,
    pub examples: Vec<String>,
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
    pub language : String,
    pub forms : Vec<String>,
    pub sense_key : Option<String>,
    pub subcats : Vec<String>,
    pub subcat_refs : Vec<String>,
    pub importance : Option<u32>,
    pub pronunciations : Vec<Pronunciation>,
    pub entry_no : u32
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Relation {
    pub src_word : Option<String>,
    pub trg_word : Option<String>,
    pub rel_type : String,
    pub target : String
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Pronunciation {
    pub value : String,
    pub variety : Option<String>
}

