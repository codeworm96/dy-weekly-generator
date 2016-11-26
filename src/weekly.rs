use std::collections::HashMap;
use std::mem;
use std::fs::File;
use std::io::prelude::*;

use yaml_rust::YamlLoader;
use hyper;

pub enum Error {
    ConfigErr,
    RequestErr(hyper::error::Error),
    FetchErr,
    JsonParseErr,
    IOErr,
}

enum EntryType { Draft, Topic }

pub struct Entry {
    name: String,
    kind: EntryType,
    link: Option<String>,
    description: Option<String>,
    quote: Option<String>,
    cc: Vec<String>,
    // TODO: tag? keyword?
}

pub struct Weekly {
    entries: HashMap<String, Entry>,
}

impl Entry {
    fn parse(yaml: &str) -> Option<Entry> {
        YamlLoader::load_from_str(yaml).ok().and_then(|docs| {
            docs.iter().next().and_then(|doc| {
                let name = doc["name"].as_str().map(|s| { s.to_string() });
                let kind = match doc["type"].as_str() {
                    Some("draft") => Some(EntryType::Draft),
                    Some("topic") => Some(EntryType::Topic),
                    Some(_) => None,
                    None => Some(EntryType::Draft),
                };
                let link = doc["link"].as_str().map(|s| { s.to_string() });
                let description = doc["description"].as_str().map(|s| { s.to_string() });
                let quote = doc["quote"].as_str().map(|s| { s.to_string() });
                let mut cc = Vec::new();
                for person in doc["cc"].as_vec().unwrap_or(&Vec::new()) {
                    match person.as_str() {
                        Some(c) => cc.push(c.to_string()),
                        None => {}
                    }
                }

                match (name, kind) {
                    (Some(name), Some(kind)) => Some(Entry {
                        name: name,
                        kind: kind,
                        link: link,
                        description: description,
                        quote: quote,
                        cc: cc,
                    }),
                    _ => None,
                }
            })
        })
    }
    
    fn field_append(a: &mut Option<String>, b: &mut Option<String>) {
        match mem::replace(b, None) {
            Some(s2) => {
                if a.is_some() {
                    a.as_mut().map(|s1| { s1.push_str(&s2) });
                } else {
                    mem::replace(a, Some(s2));
                }
            }
            None => {}
        }
    }

    fn merge(&mut self, mut other: Entry) {
        assert_eq!(self.name, other.name);
        self.kind = other.kind;
        Self::field_append(&mut self.link, &mut other.link);
        Self::field_append(&mut self.description, &mut other.description);
        Self::field_append(&mut self.quote, &mut other.quote);
        self.cc.append(&mut other.cc);
    }

    fn render(&self, file: &mut File) -> Result<(), Error> {
        try!(write!(file, "- ").map_err(|_| { Error::IOErr }));
        match self.link.as_ref() {
            Some(link) => try!(write!(file, "[{}]({})", self.name, link).map_err(|_| { Error::IOErr })),
            None => try!(write!(file, "{}", self.name).map_err(|_| { Error::IOErr })),
        }
        match self.description.as_ref() {
            Some(desc) => try!(write!(file, ", {}\n", desc).map_err(|_| { Error::IOErr })),
            None => try!(write!(file, "\n").map_err(|_| { Error::IOErr })),
        }
        match self.quote.as_ref() {
            Some(quote) => {
                for line in quote.lines() {
                    try!(write!(file, " > {}\n", line).map_err(|_| { Error::IOErr }));
                }
            }
            None => {}
        }
        if self.cc.len() > 0 {
            let cc_list: Vec<_> = self.cc.iter().map(|person| { format!("[@{}][{}]", person, person) }).collect();
            try!(write!(file, "{}\n", cc_list.join(", ")).map_err(|_| { Error::IOErr }));
        }
        Ok(())
    }
}

impl Weekly {
    pub fn new() -> Weekly {
        Weekly {
            entries: HashMap::new(),
        }
    }

    pub fn parse(&mut self, yaml: &str) {
        let entry = Entry::parse(yaml);
        match entry {
            Some(e) => {
                if let Some(ent) = self.entries.get_mut(&e.name) {
                    ent.merge(e);
                    return;
                }
                self.entries.insert(e.name.clone(), e);
            }
            None => {},
        }
    }

    pub fn render(&self, mut file: File) -> Result<(), Error> {
        for entry in self.entries.values() {
            try!(entry.render(&mut file));
        }
        Ok(())
    }
}