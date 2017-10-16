use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::io::{BufRead,BufReader};
use wordnet::{WNKey, WordNetLoadError, WordNet};
use std::str::FromStr;

pub fn load_omwn<P: AsRef<Path>>(p : P, wordnet : &WordNet)
        -> Result<HashMap<WNKey, Vec<String>>, WordNetLoadError> {
    let file = BufReader::new(File::open(p)?);
    let mut result = HashMap::new();
    for line in file.lines () {
        let line = line?;
        let mut elems = line.split("\t");
        if !line.starts_with("#") && line.len() > 0 {
            match elems.next() {
                Some(wn30) => {
                   let wn30key = WNKey::from_str(wn30)?;
                   elems.next().map(|t| {
                        elems.next().map(|v| {
                            if t.ends_with("lemma") {
                                wordnet.get_id_by_old_id("wn30", &wn30key)
                                  .expect("Need WordNet 3.0 mapping to load OMWN")
                                  .map(|id| {
                                    result.entry(id.clone())
                                        .or_insert_with(|| Vec::new())
                                        .push(v.to_string());
                                });
                            } 
                        })
                    });
                },
                None => {}
            }
        }
    }
    Ok(result)
}

