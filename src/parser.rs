use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

const SAME_TYPE: &str = "sametypesequence";

#[derive(Debug)]
pub struct Description {
    pub dict: HashMap<String, String>,
}

impl Description {
    pub fn new(info: &str) -> Self {
        let f = File::open(info).unwrap();
        let reader = BufReader::new(f);

        let mut lines: Vec<String> = Vec::new();
        for line in reader.lines() {
            lines.push(line.unwrap().to_string());
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

        Description { dict: d }
    }

    pub fn is_same_type_sequence(&self) -> bool {
        self.dict.contains_key(SAME_TYPE)
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Discription: {{")?;
        for (k, v) in self.dict.iter() {
            write!(f, "{} : {}", k, v)?;
        }
        write!(f, "}}")
    }
}

// Word represent the dictionary unit: word
// https://github.com/huzheng001/stardict-3/blob/master/dict/doc/StarDictFileFormat#L165
// word_str;  // a utf-8 string terminated by '\0'.
// word_data_offset;  // word data's offset in .dict file
// word_data_size;  // word data's total size in .dict file
#[derive(Debug, Clone)]
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

impl Index {
    pub fn new(path: &str) -> Self {
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
        // note: one word may have mutltiple data chunk, so we hashmap: word -> [dict_chunk0, dict_chunk1...]

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

        // 2. word_data_offset; word data's offset in .dict file
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

        // 3. word_data_size;  word data's total size in .dict file
        // word_data_size should be 32-bits unsigned number in network byte order.
        // unlike word_data_offset it is *always* uint32

        let mut tmp = &self.content[self.offset..self.offset + 4];
        let num = tmp.read_u32::<BigEndian>().unwrap();

        self.offset += 4;

        new_word.size = num;

        new_word.index = self.index;
        self.index += 1;

        // FIXME: copy to word to save in lst and dict
        // a new trick to copy struct field: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#creating-instances-from-other-instances-with-struct-update-syntax
        let new2 = new_word.clone();
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
    pub content: Vec<u8>,
    pub offset: usize,
}

impl Dictionary {
    pub fn new(dict_path: &str) -> Self {
        let raw_zip = fs::read(dict_path).unwrap();
        // Vec[u8] ==> bytes array
        let mut d = GzDecoder::new(&raw_zip[..]);
        let mut buf: Vec<u8> = Vec::new();
        d.read_to_end(&mut buf).unwrap();

        let d: Dictionary = Dictionary {
            content: buf,
            offset: 0,
        };

        d
    }

    pub fn get_entry_field_size(&mut self, size: usize) -> Vec<u8> {
        let value = &self.content[self.offset..self.offset + size];
        self.offset += size;

        value.to_vec()
    }

    pub fn get_word_same_sequence(&mut self, w: &Word, info: &Description) -> HashMap<u8, Vec<u8>> {
        let mut ret: HashMap<u8, Vec<u8>> = HashMap::new();
        let sametypesequence = info.dict.get("sametypesequence").unwrap();

        let start_offset = self.offset;

        for (i, c) in sametypesequence.chars().enumerate() {
            match c {
                'W' | 'P' => {
                    //     |----------------- size -----------------------|
                    //     |                            |
                    // startOffset                    offset
                    if i == sametypesequence.len() - 1 {
                        ret.insert(
                            c as u8,
                            self.get_entry_field_size(
                                w.size as usize - (self.offset - start_offset),
                            ),
                        );
                    } else {
                        let mut tmp = &self.content[self.offset..self.offset + 4];
                        let num = tmp.read_u32::<BigEndian>().unwrap();
                        self.offset += 4;

                        ret.insert(c as u8, self.get_entry_field_size(num as usize));
                    }
                }
                // "mlgtxykwhnr"
                _ => {
                    // just like 'W' 'P'
                    if i == sametypesequence.len() - 1 {
                        ret.insert(
                            c as u8,
                            self.get_entry_field_size(
                                w.size as usize - (self.offset - start_offset),
                            ),
                        );
                    } else {
                        let mut empty = start_offset;
                        loop {
                            if empty == 0x0 {
                                break;
                            }
                            empty += 1;
                        }
                        let value = &self.content[self.offset..empty];

                        // jumper over the '\0'
                        self.offset = empty + 1;
                        ret.insert(c as u8, value.to_vec());
                    }
                }
            }
        }

        ret
    }
    pub fn get_word_non_same_sequence(&mut self, w: &Word) -> HashMap<u8, Vec<u8>> {
        let mut ret: HashMap<u8, Vec<u8>> = HashMap::new();

        let mut read_size: usize = 0;
        let start_offset = self.offset;

        while read_size < w.size as usize {
            let type_byte: u8 = self.content[self.offset];
            let type_char: char = type_byte as char;
            // jump over the type byte
            self.offset += 1;

            if type_char.is_lowercase() {
                let mut end: usize = self.offset;
                loop {
                    if end == 0x0 {
                        break;
                    }
                    end += 1
                }
                let value = &self.content[self.offset..end];
                ret.insert(type_byte, value.to_vec());

                self.offset = end + 1;
            } else {
                let mut tmp = &self.content[self.offset..self.offset + 4];
                let num = tmp.read_u32::<BigEndian>().unwrap();
                self.offset += 4;

                ret.insert(type_byte, self.get_entry_field_size(num as usize));
            }

            read_size = self.offset - start_offset;
        }

        ret
    }

    pub fn get_word(
        &mut self,
        word: &str,
        info: &Description,
        idx: &Index,
    ) -> Option<Vec<HashMap<u8, Vec<u8>>>> {
        if !idx.word_dict.contains_key(word) {
            return None;
        }

        let mut ret: Vec<HashMap<u8, Vec<u8>>> = Vec::new();

        let idexs = idx.word_dict.get(word).unwrap();
        for cur_word in idexs.iter() {
            self.offset = cur_word.offset as usize;
            if info.is_same_type_sequence() {
                let cur_value = self.get_word_same_sequence(&cur_word, info);
                ret.push(cur_value);
            } else {
                let cur_value = self.get_word_non_same_sequence(&cur_word);
                ret.push(cur_value);
            }
        }

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_description() {
        let d = Description::new("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.ifo");
        for (k, v) in d.dict.iter() {
            println!("{}==>{}", k, v);
        }

        println!("{:?}", d);
    }

    #[test]
    fn test_new_index() {
        let _ = Index::new("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.idx");
    }

    #[test]
    fn test_dict() {
        let mut bin = Dictionary::new("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.dict.dz");
        let des = Description::new("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.ifo");
        let idx = Index::new("./src/testdata/stardict-oxford-gb-2.4.2/oxford-gb.idx");

        let meaning = bin.get_word(&"congratulation", &des, &idx);
        match meaning {
            None => panic!("should search a word named hello"),
            Some(meanings) => {
                for m in meanings.iter() {
                    for (k, v) in m.iter() {
                        println!("keytype {}", *k as char);
                        println!("value string\n{}\n", std::str::from_utf8(&v).unwrap());
                    }
                }
            }
        }
    }
}
