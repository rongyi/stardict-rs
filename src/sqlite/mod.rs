#![allow(dead_code)]
use std::error::Error;

use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let conn = Connection::open(file_path)?;
        Ok(Self { conn })
    }

    pub fn create_table(&mut self) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "create table words(
                id integer primary key,
                word text not null,
                meaning text not null
        )",
            (),
        )?;

        self.conn
            .execute("create unique index idx_words on words(word)", ())?;

        Ok(())
    }

    pub fn insert_word(&mut self, word: &str, meaning: &str) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "insert into words(word, meaning) values (?1, ?2)",
            (word, meaning),
        )?;
        Ok(())
    }
    pub fn get_candidate(&mut self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut stmt = self
            .conn
            .prepare("select word from words where word like ?1")?;
        let like = format!("{}%", prefix);
        let mut rs = stmt.query(params![&like])?;
        let mut words: Vec<String> = Vec::new();

        while let Some(row) = rs.next()? {
            words.push(row.get(0)?);
        }
        Ok(words)
    }

    pub fn get_meaning(&mut self, word: &str) -> Result<String, Box<dyn Error>> {
        let mut stmt = self
            .conn
            .prepare("select meaning from words where word = ?1")?;
        let mut rs = stmt.query(params![&word])?;
        let mut meaning: String = String::new();

        while let Some(row) = rs.next()? {
            meaning = row.get(0)?;
            break;
        }

        Ok(meaning)
    }
}
