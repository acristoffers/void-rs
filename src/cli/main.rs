/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2020 Álan Crístoffer e Sousa
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

mod args;
mod store;

use args::{Arguments, Commands, Parser};
use rpassword::read_password_from_tty;

fn main() {
    let options = Arguments::parse();

    match options.command {
        Commands::Create { store_name } => loop {
            let pswd = read_password(options.password.clone());
            let pswd_confirm = read_password(options.password.clone());

            if pswd != pswd_confirm {
                println!("Passwords do not match.");
                continue;
            }

            if store::create_store(store_name, pswd).is_none() {
                std::process::exit(1);
            }

            break;
        },

        Commands::Add {
            store_path,
            files,
            internal_path,
        } => {
            let pswd = read_password(options.password);
            if store::add(store_path, internal_path, files, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::Get {
            store_path,
            internal_path,
            external_path,
        } => {
            let pswd = read_password(options.password);
            if store::get(store_path, internal_path, external_path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::LS {
            human,
            store_path,
            path,
            list,
        } => {
            let pswd = read_password(options.password);
            if store::list(store_path, path, pswd, human, list).is_none() {
                std::process::exit(1);
            }
        }

        Commands::RM { store_path, path } => {
            let pswd = read_password(options.password);
            if store::remove(store_path, path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::MetadataSet {
            store_path,
            path,
            key,
            value,
        } => {
            let pswd = read_password(options.password);
            if store::metadata_set(store_path, path, pswd, key, value).is_none() {
                std::process::exit(1);
            }
        }

        Commands::MetadataGet {
            store_path,
            path,
            key,
        } => {
            let pswd = read_password(options.password);
            if store::metadata_get(store_path, path, pswd, key).is_none() {
                std::process::exit(1);
            }
        }

        Commands::MetadataList { store_path, path } => {
            let pswd = read_password(options.password);
            if store::metadata_list(store_path, path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::MetadataRemove {
            store_path,
            path,
            key,
        } => {
            let pswd = read_password(options.password);
            if store::metadata_remove(store_path, path, pswd, key).is_none() {
                std::process::exit(1);
            }
        }

        Commands::TagAdd {
            store_path,
            path,
            tags,
        } => {
            let pswd = read_password(options.password);
            for tag in tags {
                if store::tag_add(store_path.clone(), path.clone(), pswd.clone(), tag).is_none() {
                    std::process::exit(1);
                }
            }
        }

        Commands::TagRemove {
            store_path,
            path,
            tags,
        } => {
            let pswd = read_password(options.password);
            for tag in tags {
                if store::tag_remove(store_path.clone(), path.clone(), pswd.clone(), tag).is_none()
                {
                    std::process::exit(1);
                }
            }
        }

        Commands::TagGet { store_path, path } => {
            let pswd = read_password(options.password);
            if store::tag_get(store_path, path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::TagList { store_path } => {
            let pswd = read_password(options.password);
            if store::tag_list(store_path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::TagClear { store_path, path } => {
            let pswd = read_password(options.password);
            if store::tag_clear(store_path, path, pswd).is_none() {
                std::process::exit(1);
            }
        }

        Commands::TagSearch { store_path, tags } => {
            let pswd = read_password(options.password);
            if store::tag_search(store_path, tags, pswd).is_none() {
                std::process::exit(1);
            }
        }
    }
}

fn read_password(password: Option<String>) -> String {
    match password {
        Some(pswd) => pswd,
        None => {
            let err = "Error reading password.";
            read_password_from_tty(Some("Password: ")).expect(err)
        }
    }
}
