use std::fs::File;
use std::path::Path;
use std::io::{BufReader};
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use std::collections::HashMap;
use crate::wordnet::{WNKey, WordNetLoadError, WordNetBuilder};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct GlossTagWord {
    text : String,
    lemma : Option<String>,
    sep : String,
    pos : Option<String>,
    tag : Option<String>,
    synset : Option<String>,
    glob : Option<String>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum GlossType {
    Aux, Def, Ex
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Gloss {
    words : Vec<GlossTagWord>,
    gloss_type : GlossType
}

pub type GlossTagCorpus = HashMap<WNKey, Vec<Gloss>>;

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}


fn read_glosstag_corpus<P : AsRef<Path>>(path : P,
        wordnet : &WordNetBuilder) -> Result<GlossTagCorpus, WordNetLoadError> {
    let file = BufReader::new(File::open(path)?);

    let parse = EventReader::new(file);

    let mut current_id : Option<WNKey> = None;
    let mut current_sents : Vec<Gloss> = Vec::new();
    let mut current_sent = Vec::new();
    let mut current_word = GlossTagWord { 
        text: "".to_string(), lemma: None, sep: "".to_string(),
        pos: None, tag: None, synset: None, glob: None };        
    let mut all_sents = HashMap::new();
    let mut in_glob = false;

    for e in parse {
        match e {
            Ok(XmlEvent::StartElement{ name, attributes, .. }) => {
                if name.local_name == "synset" {
                    let wn30id = attr_value(&attributes, "id")
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "synset does not have id"))?;
                    let pos = wn30id.chars().next()
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "bad wn30 id"))?;
                    let num : String = wn30id.chars().skip(1).collect();
                    //let id = WNKey::from_str(&format!("{}-{}", num ,pos))?;
                    let id = format!("{}-{}", num, pos);
                    current_id = wordnet.get_id_by_pwn30(&id)
                        .expect("Loading gloss tags without WN 3.0 index")
                        .map(|x| x.clone());
                } else if name.local_name == "gloss" {
                    current_sents = Vec::new();
                } else if name.local_name == "aux" {
                    current_sent = Vec::new();
                } else if name.local_name == "def" {
                    current_sent = Vec::new();
                } else if name.local_name == "ex" {
                    current_sent = Vec::new();
                } else if name.local_name == "wf" || name.local_name == "cf" {
                    let lemma = attr_value(&attributes, "lemma");
                    let pos = attr_value(&attributes, "pos");
                    let tag = attr_value(&attributes, "tag");
                    let sep = attr_value(&attributes, "sep")
                        .unwrap_or(" ".to_string());
                    current_word = GlossTagWord {
                        text : "".to_string(),
                        lemma: lemma,
                        pos: pos,
                        tag: tag,
                        sep: sep,
                        synset: None,
                        glob: None
                    };
                } else if name.local_name == "glob" {
                    in_glob = true;
                } else if name.local_name == "id" {
                    let sk = attr_value(&attributes, "sk") 
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "id does not have sk"))?;
                    wordnet.get_id_by_sense_key(&sk)?
                        .map(|ss| {
                            if in_glob {
                                current_word.glob = Some(ss.to_string())
                            } else {
                                current_word.synset = Some(ss.to_string()) 
                            }
                        });
                }
            },
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "synset" {
                    current_id = None;
                } else if name.local_name == "gloss" {
                    match current_id.clone() {
                        Some(ssid) => {
                            all_sents.insert(ssid.clone(),
                                current_sents.clone());
                        },
                        None => {}
                    }
                } else if name.local_name == "aux" {
                    current_sents.push(Gloss {
                        words: current_sent.clone(),
                        gloss_type: GlossType::Aux
                    });
                } else if name.local_name == "def" {
                    current_sents.push(Gloss {
                        words: current_sent.clone(),
                        gloss_type: GlossType::Def
                    });
                } else if name.local_name == "ex" {
                    current_sents.push(Gloss {
                        words: current_sent.clone(),
                        gloss_type: GlossType::Ex
                    });
                } else if name.local_name == "glob" {
                    in_glob = false;
                } else if name.local_name == "wf" || name.local_name == "cf" {
                    current_sent.push(current_word.clone());
                }
                
            },
            Ok(XmlEvent::Characters(s)) => {
                current_word.text = s;
            },
            Ok(_) => {},
            Err(e) => { return Err(WordNetLoadError::Xml(e)); }
        }
    }
    Ok(all_sents)
}

pub fn build_glosstags(wordnet : &mut WordNetBuilder)
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


