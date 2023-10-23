#![allow(dead_code)]
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{BufRead, Cursor, Read},
};

#[derive(Debug)]
pub struct Dictionary<'a> {
    data: Cursor<Vec<u8>>,
    same_type_sequence: &'a str,
}

impl<'a> Dictionary<'a> {
    pub fn new(file_path: &str, same_type_sequence: &'a str) -> Result<Self, Box<dyn Error>> {
        let f = fs::File::open(file_path)?;
        let mut zr = flate2::read::GzDecoder::new(f);
        let mut buf = Vec::new();
        zr.read_to_end(&mut buf)?;

        Ok(Dictionary {
            data: Cursor::new(buf),
            same_type_sequence,
        })
    }

    pub fn get_oxford(&mut self, offset: u64, sz: u64) -> Result<String, Box<dyn Error>> {
        let dict = self.get(offset, sz)?;
        let meaning: Vec<String> = dict
            .iter()
            .map(|(_, value)| String::from_utf8(value.clone()).unwrap())
            .collect();

        Ok(meaning.join("\n"))
    }

    pub fn get(&mut self, offset: u64, sz: u64) -> Result<HashMap<char, Vec<u8>>, Box<dyn Error>> {
        self.data.set_position(offset);
        let mut ret = HashMap::new();
        for (i, c) in self.same_type_sequence.chars().enumerate() {
            if i == self.same_type_sequence.len() - 1 {
                let read_sz = self.data.position() - offset;
                let left_sz = sz - read_sz;
                let mut buf = vec![0x00u8; left_sz as usize];
                self.data.read_exact(&mut buf)?;
                ret.insert(c, buf);
            } else {
                match c {
                    'W' | 'P' => {
                        let mut next_sz_buf = [0x00u8; 4];
                        self.data.read_exact(&mut next_sz_buf)?;
                        let next_sz = u32::from_be_bytes(next_sz_buf);
                        let mut buf = vec![0x00u8; next_sz as usize];
                        self.data.read_exact(&mut buf)?;
                        ret.insert(c, buf);
                    }
                    _ => {
                        let mut buf = vec![];
                        let sz = self.data.read_until(0x00u8, &mut buf)?;
                        if sz > 0 {
                            buf.pop();
                        }
                        ret.insert(c, buf);
                    }
                }
            }
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dictionary() {
        let dict = Dictionary::new(
            "testdata/stardict-oxford-gb-formated-2.4.2/oxford-gb-formated.dict.dz",
            "m",
        );
        assert!(dict.is_ok());
        let mut dict = dict.unwrap();
        let explain = dict.get(5802983, 43);
        assert!(explain.is_ok());
        let explain = explain.unwrap();
        for (_, meaning) in &explain {
            println!("{}", String::from_utf8(meaning.clone()).unwrap());
        }
    }
}
