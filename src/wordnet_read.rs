//! Code for loading wordnets from disk
use crate::omwn::load_omwn;
use std::collections::HashMap;
use std::fs::{File};
use std::io::{BufRead,BufReader};
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};
use crate::links::{load_links};
use crate::wordnet::{WordNetLoadError,WordNetBuilder,WNKey, WordNet};
use crate::wordnet_model::{Sense,Synset,Relation,PartOfSpeech,Pronunciation};
use std::str::FromStr;
use xml::attribute::OwnedAttribute;
use crate::glosstag::build_glosstags;


fn unmap_sense_key(sk : &str) -> String {
    match sk.find("-")  {
        Some(i) => {
            let sk = &sk[i+1..];
            sk.replace("__", "%").replace("-ap-", "'").replace("-sl-", "/").replace("-ex-", "!").replace("-cm-",",")
        },
        None => "".to_string()
    }
}


fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

fn clean_id(s : &str) -> Result<WNKey, WordNetLoadError> {
    if s.starts_with("wn31-") {
        let s2 : String = s.chars().skip(5).collect();
        //WNKey::from_str(&s2)
        Ok(s2)
    } else {
        Ok(s.to_string())
    }
}

pub struct LoadConfiguration {
    tabs : bool,
    glosstags : bool,
    omwn : bool,
    links : bool
}

/// Load a Princeton WordNet-style GWN XML file and associated elements into
/// the database
pub fn load_pwn<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
    load(path, &LoadConfiguration {
        tabs: true,
        glosstags: false,
        omwn: true,
        links : true
    })
}

/// Load a Global WordNet XML file without any of the other associated elements
pub fn load_gwn<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
    load(path, &LoadConfiguration {
        tabs: false,
        glosstags: false,
        omwn: false,
        links : false
    })
}

/// Load the English WordNet
pub fn load_enwn<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
    load(path, &LoadConfiguration {
        tabs: false,
        glosstags: false,
        omwn: false,
        links: false
    })
}


fn load<P : AsRef<Path>>(path : P, 
                                 config : &LoadConfiguration) -> Result<WordNet, WordNetLoadError> {
    let mut wordnet = WordNetBuilder::new()?;
    load_xml(path, &mut wordnet)?;
    if config.tabs {
        build_tabs(&mut wordnet)?;
    }
    if config.glosstags {
        build_glosstags(&mut wordnet)?;
    }
    if config.omwn {
        build_omwn(&mut wordnet)?;
    }
    if config.links {
        load_links(&mut wordnet)?;
    }
    wordnet.finalize()
}

fn load_xml<P : AsRef<Path>>(path : P, 
                                 wordnet : &mut WordNetBuilder) -> Result<(), WordNetLoadError> {
    let file = BufReader::new(File::open(path)?);

    let parse = EventReader::new(file);

    let mut lexical_entry_id : Option<String> = None;
    let mut entry_lemma = None;
    let mut sense_keys = HashMap::new();
    let mut entry_id_to_lemma = HashMap::new();
    let mut synset_to_entry = HashMap::new();
    let mut sense_to_lemma = HashMap::new();
    let mut sense_to_synset = HashMap::new();
    let mut entry_id_to_forms = HashMap::new();
    let mut entry_synset_to_sense = HashMap::new();
    let mut subcats = HashMap::new();
    let mut subcat_refs = HashMap::new();
    let mut synset = None;
    let mut synset_id = None;
    let mut synset_ili_pos_subject = None;
    let mut in_def = false;
    let mut definition = None;
    let mut in_example = false;
    let mut examples = Vec::new();
    let mut synsets : HashMap<String, Synset> = HashMap::new();
    let mut relations : HashMap<WNKey,Vec<Relation>> = HashMap::new();
    let mut language = "en".to_string();
    let mut entries_read = 0;
    let mut senses_read = 0;
    let mut sense_orders = HashMap::new();
    let mut lemma_orders = HashMap::new();
    let mut variety = None;
    let mut in_pron = false;
    let mut pronunciations = Vec::new();
    let mut entry_id_to_prons = HashMap::new();

    for e in parse {
        match e {
            Ok(XmlEvent::StartElement{ name, attributes, .. }) => {
                if name.local_name == "Lexicon" {
                    match attr_value(&attributes, "language") {
                        Some(l) => {
                            language = l;
                        },
                        None =>
                            return Err(WordNetLoadError::Schema(
                                    "Lexicon does not have a language"))
                    }
                } else if name.local_name == "LexicalEntry" {
                    entries_read += 1;
                    if entries_read % 100000 == 0 {
                        eprintln!("Read {}", entries_read);
                    }
                    match attr_value(&attributes, "id") {
                        Some(id) => {
                            lexical_entry_id = Some(id)
                        },
                        None => {
                            return Err(WordNetLoadError::Schema(
                                "LexicalEntry does not have an ID"));
                        }
                    }
                    senses_read = 0;
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
                } else if name.local_name == "Form" {
                    if let Some(f) = attr_value(&attributes, "writtenForm") {
                        let entry_id = lexical_entry_id.clone()
                            .ok_or(WordNetLoadError::Schema(
                                        "Form outside of LexicalEntry"))?;
                        entry_id_to_forms.entry(entry_id)
                            .or_insert_with(|| Vec::new())
                            .push(f);
                    }
                } else if name.local_name == "Sense" {
                    let entry_id = match lexical_entry_id {
                        Some(ref i) => i.clone(),
                        None => {
                            return Err(WordNetLoadError::Schema(
                                "Sense outside of LexicalEntry"))
                        }
                     };
                     let target = clean_id(&attr_value(&attributes, "synset")
                        .ok_or_else(|| WordNetLoadError::Schema(
                                "Sense does not have a synset"))?)?;
                     let sense_id = attr_value(&attributes, "id")
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "Sense without id"))?;
                     match attr_value(&attributes, "identifier") {
                        Some(i) => {
                            sense_keys.entry(entry_id.clone())
                                .or_insert_with(|| HashMap::new())
                                .insert(target.clone(), i);
                        },
                        None => {
                            if sense_id.contains("__") {
                                sense_keys.entry(entry_id.clone())
                                    .or_insert_with(|| HashMap::new())
                                    .insert(target.clone(), 
                                            unmap_sense_key(&sense_id));
                            }
                        }
                     };
                     synset = Some(target.clone());
                     synset_to_entry.entry(target.clone())
                        .or_insert_with(|| Vec::new())
                        .push((entry_id, language.clone()));
                     // Replace with members for future versions
                     let word = entry_lemma.clone()
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "SenseRelation before Lemma"))?;
                     match attr_value(&attributes, "subcat") {
                         Some(scs) => {
                             subcat_refs.insert((word.clone(), target.clone()),
                                scs.split(" ").map(|s| s.to_string()).
                                collect::<Vec<String>>());
                         },
                         None => {}
                     }
                     sense_to_lemma.insert(sense_id.clone(), word.clone());
                     sense_to_synset.insert(sense_id.clone(), target.clone());
                     senses_read += 1;
                     if sense_id.len() > 2 {
                         if let Ok(word_order) = sense_id[(sense_id.len() - 2)..(sense_id.len())].parse::<u32>() {
                            lemma_orders.insert((word.clone(), target.clone()), word_order);
                         }
                     }
                     entry_synset_to_sense.insert((word.clone(), target.clone()), sense_id);
                     sense_orders.insert((word, target), senses_read);
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
                    match lexical_entry_id {
                        Some(ref lid) => {
                            let entry_id = lid.clone();
                            let subcat = attr_value(&attributes, "subcategorizationFrame")
                                .ok_or_else(|| WordNetLoadError::Schema(
                                    "SyntacticBehaviour has no subcategorizationFrame"))?;
                            match attr_value(&attributes, "senses") {
                                Some(sense_list) => 
                                    for sense_id in sense_list.split(" ") {
                                        subcats.entry(sense_id.to_string())
                                            .or_insert_with(|| Vec::new())
                                            .push(subcat.clone())
                                    },
                                None => 
                                    subcats.entry(entry_id)
                                        .or_insert_with(|| Vec::new())
                                        .push(subcat)
                            }
                        },
                        None => {
                            let id = attr_value(&attributes, "id")
                                .ok_or_else(|| WordNetLoadError::Schema(
                                    "SyntacticBehaviour has no ID"))?;
                            let subcat = attr_value(&attributes, "subcategorizationFrame")
                                .ok_or_else(|| WordNetLoadError::Schema(
                                    "SyntacticBehaviour has no subcategorizationFrame"))?;
                            for synset in synsets.iter_mut() {
                                for sense in synset.1.lemmas.iter_mut() {
                                    if sense.subcat_refs.iter().any(|sr| *sr == id) {
                                        sense.subcats.push(subcat.clone());
                                    }
                                }
                            }
                        }
                    }
                } else if name.local_name == "Synset" {
                    entries_read += 1;
                    if entries_read % 100000 == 0 {
                        eprintln!("Read {}", entries_read);
                    }
                    synset_id = Some(clean_id(&attr_value(&attributes, "id")
                        .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have an id"))?)?);
                    synset_ili_pos_subject = Some((
                        attr_value(&attributes, "ili")
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "Synset does not have ILI"))?,
                        attr_value(&attributes, "partOfSpeech")
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "Synset does not have part of speech"))?,
                        attr_value(&attributes, "lexfile")
                        .ok_or_else(|| WordNetLoadError::Schema(
                            "Synset does not have subject"))?));
                } else if name.local_name == "Definition" {
                    in_def = true;
                } else if name.local_name == "Example" {
                    in_example = true;
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
                } else if name.local_name == "Pronunciation" {
                    variety = attr_value(&attributes, "variety");
                    in_pron = true;
                }
            },
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "LexicalEntry" {
                    entry_id_to_prons.insert(lexical_entry_id.unwrap(),
                                             pronunciations.clone());
                    pronunciations = Vec::new();
                    lexical_entry_id = None;
                    entry_lemma = None;
                } else if name.local_name == "Sense" {
                    synset = None;
                } else if name.local_name == "Synset" {
                    let defn = definition.ok_or(
                        WordNetLoadError::Schema(
                            "Synset without definition"))?;
                    let ssid = synset_id.unwrap();
                    //let entries = Vec::new();
                    let mut entries : Vec<Sense> = synset_to_entry.get(&ssid)
                        .map(|x| x.clone())
                        .unwrap_or_else(|| Vec::new())
                        .iter()
                        .map(|x| {
                            let mut synset_subcats = Vec::new();
                            synset_subcats.extend(subcats.get(&x.0)
                                                  .map(|x| x.clone())
                                                  .unwrap_or_else(|| Vec::new())
                                                  .into_iter());
                                                  
                            synset_subcats.extend(
                                    entry_synset_to_sense.get(&(entry_id_to_lemma.get(&x.0).unwrap().clone(), ssid.clone()))
                                        .and_then(|x| subcats.get(&x.to_string()))
                                        .map(|x| x.clone())
                                        .unwrap_or_else(|| Vec::new())
                                        .into_iter());
                            let lemma = entry_id_to_lemma.get(&x.0)
                                    .expect("Entry must have lemma");
                            let entry_no = match x.0.chars().next_back() {
                                Some('1') => 1,
                                Some('2') => 2,
                                Some('3') => 3,
                                Some('4') => 4, // This could cause bugs
                                _ => 0
                            };

                            Sense {
                                lemma: lemma.clone(),
                                language: x.1.clone(),
                                forms: entry_id_to_forms.get(&x.0)
                                    .map(|x| x.clone())
                                    .unwrap_or_else(|| Vec::new()),
                                sense_key: sense_keys.get(&x.0).and_then(
                                    |x| x.get(&ssid).map(|y| y.clone())),
                                subcats: synset_subcats,
                                subcat_refs: subcat_refs.remove(
                                    &(lemma.clone(), ssid.clone()))
                                    .unwrap_or_else(|| Vec::new()),
                                importance: sense_orders.get(&(lemma.clone(), ssid.clone())).map(|y| *y),
                                pronunciations: entry_id_to_prons.get(&x.0)
                                    .map(|x| x.clone()).unwrap_or_else(|| Vec::new()),
                                entry_no
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
                    entries.sort_by_key(|entry| {
                       lemma_orders.get(&(entry.lemma.clone(), ssid.clone())).map(|x| *x).unwrap_or(100)
                    });
                    synsets.insert(ssid.clone(),
                        Synset {
                            definition: defn,
                            examples: examples.clone(),
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
                    examples.clear();
                } else if name.local_name == "Definition" {
                    in_def = false;
                } else if name.local_name == "Example" {
                    in_example = false;
                } else if name.local_name == "Pronunciation" {
                    variety = None;
                    in_pron = false;
                }
            },
            Ok(XmlEvent::Characters(s)) => {
                if in_def {
                    definition = Some(s);
                } else if in_example {
                    examples.push(s);
                } else if in_pron {
                    let v = variety;
                    variety = None;
                    pronunciations.push(Pronunciation { value:s, variety: v });
                }
            },
            Ok(_) => {},
            Err(e) => { return Err(WordNetLoadError::Xml(e)); }
        }
    }
    wordnet.set_synsets(synsets)
}

fn build_tab<P : AsRef<Path>>(file : P, 
    index : &str,
    wordnet : &mut WordNetBuilder) -> Result<(),WordNetLoadError> {
    let file = BufReader::new(File::open(file)?);
    let mut values = Vec::new();
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
//                                values.push((WNKey::from_str(id)?, wnid));
                                values.push((id.to_string(), wnid));
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
    wordnet.set_old_ids(index, values)?;
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

fn build_tabs(wordnet : &mut WordNetBuilder) -> Result<(),WordNetLoadError> {
    for tab in ["pwn15", "pwn16", "pwn17", "pwn171", "pwn20",
                "pwn21", "pwn30"].iter() {
        eprintln!("Loading Tab {}", tab);
        let path = format!("data/ili-map-{}.tab", tab);
        build_tab(path, tab, wordnet)?;
    }
    Ok(())
}

fn build_omwn(wordnet : &mut WordNetBuilder) -> Result<(), WordNetLoadError> {
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



