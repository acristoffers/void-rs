/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod de;
mod en;
mod fr;
mod pt;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Language {
    English,
    Portuguese,
    French,
    German,
}

#[derive(Debug, Clone)]
pub struct I18N {
    ts: HashMap<String, String>,
}

impl I18N {
    pub fn new(lang: Language) -> I18N {
        let mut obj = I18N { ts: HashMap::new() };
        obj.set_language(lang);
        obj
    }

    pub fn set_language(&mut self, lang: Language) {
        self.ts = match lang {
            Language::English => en::translations(),
            Language::Portuguese => pt::translations(),
            Language::French => fr::translations(),
            Language::German => de::translations(),
        }
    }

    pub fn tr(&self, key: &str) -> String {
        self.ts
            .get(&key.to_string())
            .unwrap_or(&key.to_string())
            .clone()
    }
}
