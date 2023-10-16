#![allow(dead_code)]
use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Debug)]
pub struct Info {
    data: HashMap<String, String>,
}

impl Info {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(file_path)?;
        let data = content
            .split("\n")
            .skip(1)
            .map(|line| {
                line.split("=")
                    .map(|s| String::from(s.trim()))
                    .collect::<Vec<String>>()
            })
            .filter(|lst| lst.len() == 2)
            .map(|mut lst| (lst.swap_remove(0), lst.swap_remove(0)))
            .collect::<HashMap<String, String>>();
        Ok(Info { data })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_info() {
        let info = Info::new("testdata/stardict-oxford-gb-formated-2.4.2/oxford-gb-formated.ifo");
        assert!(info.is_ok());
        let info = info.unwrap();
        println!("{:?}", info);
    }
}
