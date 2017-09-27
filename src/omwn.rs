use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::io::{BufRead,BufReader};
use wordnet::{WNKey, WordNetLoadError};
use std::str::FromStr;

pub fn load_omwn<P: AsRef<Path>>(p : P, wn30s : &HashMap<WNKey, WNKey>) 
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
                            if t.contains("lemma") {
                                wn30s.get(&wn30key).map(|id| {
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

