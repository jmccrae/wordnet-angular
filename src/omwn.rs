use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::io::{BufRead,BufReader,Error};

pub fn load_omwn<P: AsRef<Path>>(p : P, wn30s : HashMap<String, String>) 
        -> Result<HashMap<String, Vec<String>>, Error> {
    let file = BufReader::new(File::open(p)?);
    let mut result = HashMap::new();
    for line in file.lines () {
        let line = line?;
        let mut elems = line.split("\t");
        elems.next().map(|wn30| {
            elems.next().map(|t| {
                elems.next().map(|v| {
                    if t.contains("lemma") {
                        wn30s.get(wn30).map(|id| {
                            result.entry(id.clone())
                                .or_insert_with(|| Vec::new())
                                .push(v.to_string());
                        });
                    } 
                })
            })
        });
    }
    Ok(result)
}

