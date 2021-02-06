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

use args::{Arguments, Commands, StructOpt};
use rpassword::read_password_from_tty;

fn main() {
    let options = Arguments::from_args();

    match options.command {
        Commands::Create { store_name } => {
            let name = store_name;
            let pswd = read_password(options.password);
            if let None = store::create_store(name, pswd) {
                std::process::exit(1);
            }
        }

        Commands::Add {
            store_path,
            files,
            internal_path,
        } => {
            let pswd = read_password(options.password);
            if let None = store::add(store_path, internal_path, files, pswd) {
                std::process::exit(1);
            }
        }

        Commands::Get {
            store_path,
            internal_path,
            external_path,
        } => {
            let pswd = read_password(options.password);
            if let None = store::get(store_path, internal_path, external_path, pswd) {
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
            if let None = store::list(store_path, path, pswd, human, list) {
                std::process::exit(1);
            }
        }

        Commands::RM { store_path, path } => {
            let pswd = read_password(options.password);
            if let None = store::remove(store_path, path, pswd) {
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
