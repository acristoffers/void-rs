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
    #[structopt(about = "Creates a new store")]
    Create {
        #[structopt(help = "Path to the store folder")]
        store_name: String,
    },

    #[structopt(about = "Adds a file or folder to the store")]
    Add {
        #[structopt(help = "store", short = "s", help = "Path to the store folder")]
        store_path: String,

        #[structopt(help = "Path in the store where it will be saved")]
        internal_path: String,

        #[structopt(required = true, help = "List of files to insert")]
        files: Vec<String>,
    },

    #[structopt(about = "Get a file or folder from the store (unencrypts it)")]
    Get {
        #[structopt(long = "store", short = "s", help = "Path to the store folder")]
        store_path: String,

        #[structopt(help = "Path in the store where it will be saved")]
        internal_path: String,

        #[structopt(help = "Local path where to decrypt")]
        external_path: String,
    },

    #[structopt(about = "Removes a file or folder from the store")]
    RM {
        #[structopt(help = "store", short = "s", about = "Path to the store folder")]
        store_path: String,

        #[structopt(help = "Path of file or folder to remove from store")]
        path: String,
    },

    #[structopt(about = "List files in the store")]
    LS {
        #[structopt(short = "v", help = "Prints sizes")]
        verbose: bool,

        #[structopt(short = "h", help = "Prints human-readable sizes")]
        human: bool,

        #[structopt(long = "store", short = "s", help = "Path to the store folder")]
        store_path: String,

        #[structopt(help = "Path to list")]
        path: String,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Void", about = "Encrypted file store.")]
pub struct Arguments {
    #[structopt(subcommand)]
    pub command: Commands,

    #[structopt(global = true, long = "password", short = "p", help = "Store password")]
    pub password: Option<String>,
}
