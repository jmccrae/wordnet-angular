use omwn::load_omwn;
use std::collections::HashMap;
use std::fs::{File};
use std::io::{BufRead,BufReader};
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};
use links::{load_links};
use wordnet::{WordNetLoadError,WordNetBuilder,Sense,Synset,Relation,WNKey,
WordNet,PartOfSpeech};
use std::str::FromStr;
use xml::attribute::OwnedAttribute;

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

fn clean_id(s : &str) -> Result<WNKey, WordNetLoadError> {
    let s2 : String = s.chars().skip(5).collect();
    WNKey::from_str(&s2)
}

pub fn load_pwn<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
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
    let mut subcats = HashMap::new();
    let mut synset = None;
    let mut synset_id = None;
    let mut synset_ili_pos_subject = None;
    let mut in_def = false;
    let mut definition = None;
    let mut synsets = HashMap::new();
    let mut relations : HashMap<WNKey,Vec<Relation>> = HashMap::new();

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
                                language: "en".to_string(),
                                forms: entry_id_to_forms.get(x)
                                    .map(|x| x.clone())
                                    .unwrap_or_else(|| Vec::new()),
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
    let mut wordnet = WordNetBuilder::new()?;
    wordnet.set_synsets(synsets)?;
    build_tabs(&mut wordnet)?;
    //build_glosstags(&mut wordnet)?;
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

    wordnet.finalize()
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
                                values.push((WNKey::from_str(id)?, wnid));
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



