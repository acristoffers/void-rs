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

pub use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum Commands {
    Create {
        store_name: String,
    },
    Add {
        #[structopt(long = "store", short = "s")]
        store_path: String,
        internal_path: String,
        #[structopt(required = true)]
        files: Vec<String>,
    },
    Get {
        #[structopt(long = "store", short = "s")]
        store_path: String,
        internal_path: String,
        external_path: String,
    },
    RM {
        #[structopt(long = "store", short = "s")]
        store_path: String,
        path: String,
    },
    LS {
        #[structopt(short = "v")]
        verbose: bool,
        #[structopt(short = "h")]
        human: bool,
        #[structopt(long = "store", short = "s")]
        store_path: String,
        path: String,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Void", about = "Encrypted file store.")]
pub struct Arguments {
    #[structopt(subcommand)]
    pub command: Commands,

    #[structopt(global = true, long = "password", short = "p")]
    pub password: Option<String>,
}
