use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;

#[derive(Debug)]
pub struct Description {
    pub content: String,
    pub dict: HashMap<String, String>,
}

pub fn new_description(info: &str) -> Description {
    let ct = fs::read_to_string(info).unwrap();
    let mut lines: Vec<String> = Vec::new();
    for line in ct.split("\n") {
        lines.push(line.to_string());
    }

    let mut d: HashMap<String, String> = HashMap::new();

    lines.iter().skip(1).for_each(|s| {
        let mut kv: Vec<String> = Vec::new();

        for w in s.split("=") {
            kv.push(w.to_string());
        }

        if kv.len() != 2 {
            return;
        }

        let k = kv[0].trim();
        let v = kv[1].trim().to_string();
        d.entry(k.to_string()).or_insert(v);
    });

    Description {
        content: ct,
        dict: d,
    }
}

impl ToString for Description {
    fn to_string(&self) -> String {
        let mut ret: String = String::new();
        for (k, v) in self.dict.iter() {
            let s = format!("{}=>{}", k, v);
            ret.push_str(&s);
            ret.push_str("\n");
        }
        ret.push_str("\n");

        ret
    }
}

pub struct Word {
    pub w: String,
    pub offset: u32,
    pub size: u32,
    pub index: u32,
}

pub struct Index {
    pub content: Vec<u8>,
    pub offset: usize,
    pub index: u32,
    pub index_bits: u32,

    pub word_dict: HashMap<String, Vec<Word>>,
    pub word_lst: Vec<Word>,
    pub parsed: bool,
}

pub fn new_index(path: &str) -> Index {
    let c = fs::read(path).unwrap();

    let mut ret = Index {
        offset: 0,
        index: 0,
        // stardict 2.4 version is hardcoded to 32
        index_bits: 32,
        word_lst: Vec::new(),
        word_dict: HashMap::new(),
        parsed: false,
        content: c,
    };
    ret.parse();

    ret
}
impl Index {
    fn parse(&mut self) {
        if self.parsed {
            return;
        }

        loop {
            match self.next_word() {
                Some(_) => continue,
                None => break,
            }
        }

        self.parsed = true;
    }

    fn next_word(&mut self) -> Option<String> {
        if self.offset == self.content.len() {
            return None;
        }

        // format:
        // word_str;  // a utf-8 string terminated by '\0'.
        // word_data_offset;  // word data's offset in .dict file
        // word_data_size;  // word data's total size in .dict file

        // 1. word_str;  // a utf-8 string terminated by '\0'.
        // we don't need this '\0'
        // word end with '0'
        let mut empty_tag = self.offset;
        loop {
            if self.content[empty_tag] == 0x0 {
                break;
            }
            empty_tag += 1;
        }

        let mut new_word: Word = Word {
            w: "todo".to_string(),
            offset: 0,
            size: 0,
            index: 0,
        };

        // ignore the trailing '0'
        let word = &self.content[self.offset..empty_tag];
        let ws = std::str::from_utf8(&word).unwrap();
        new_word.w = ws.to_string();

        // jump over this '00'
        self.offset = empty_tag + 1;

        // 2. word_data_offset;  // word data's offset in .dict file
        if self.index_bits == 64u32 {
            let mut tmp = &self.content[self.offset..self.offset + 8];
            let num = tmp.read_u64::<BigEndian>().unwrap();

            self.offset += 8;
            new_word.offset = num as u32;
        } else {
            // u32
            let mut tmp = &self.content[self.offset..self.offset + 4];
            let num = tmp.read_u32::<BigEndian>().unwrap();

            self.offset += 4;
            new_word.offset = num;
        }

        // word_data_size;  // word data's total size in .dict file
        // word_data_size should be 32-bits unsigned number in network byte order.
        // unlike word_data_offset it is *always* uint32

        let mut tmp = &self.content[self.offset..self.offset + 4];
        let num = tmp.read_u32::<BigEndian>().unwrap();

        self.offset += 4;

        new_word.size = num;

        new_word.index = self.index;
        self.index += 1;

        // FIXME: copy to word to save in lst and dict
        let new2 = Word {
            w: new_word.w.clone(),
            offset: new_word.offset,
            size: new_word.size,
            index: new_word.index,
        };
        let ret = new_word.w.clone();

        self.word_lst.push(new_word);

        self.word_dict
            .entry(ws.to_string())
            .or_insert(Vec::new())
            .push(new2);

        Some(ret)
    }
}

pub struct Dictionary {
    pub info: Description,
    pub index: Index,
    pub content: Vec<u8>,
    pub offset: usize,
}

fn new_dictionary(info_path: &str, idx_path: &str, dict_path: &str) -> Dictionary {
    let raw_zip = fs::read(dict_path).unwrap();
    let raw_zip = fs::read_to_string(dict_path).unwrap();
    let mut d = GzDecoder::new(raw_zip.as_bytes());
    let mut s = String::new();
    d.read_to_string(&mut s).unwrap();
    println!("here");

    let d: Dictionary = Dictionary {
        info: new_description(info_path),
        index: new_index(idx_path),
        content: s.as_bytes().to_vec(),
        offset: 0,
    };

    d
}

impl Dictionary {}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_new() {
        let d = new_description("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.ifo");
        for (k, v) in d.dict.iter() {
            println!("{}==>{}", k, v);
        }

        println!("{:?}", d);
    }
    #[test]
    fn test_string() {
        let d = new_description("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.ifo");
        let s = d.to_string();
        println!("{}", s);
    }

    #[test]
    fn test_new_index() {
        let _ = new_index("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.idx");
    }
    #[test]
    fn test_parse() {
        let idx = new_index("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.idx");
        if idx.word_lst.len() != 39429 {
            panic!("parse index fail");
        }
    }

    #[test]
    fn test_dict() {
        let _ = new_dictionary(
            "./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.ifo",
            "./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.idx",
            "./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.dict.dz",
        );
    }
}
