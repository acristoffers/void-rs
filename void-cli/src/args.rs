/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use clap::CommandFactory;
pub use clap::Parser;

#[derive(Debug, Parser)]
pub enum Commands {
    /// Creates a new store
    #[command()]
    Create {
        /// Path to the store folder
        #[arg()]
        store_name: String,
    },

    /// Adds a file or folder to the store
    #[command()]
    Add {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path in the store where it will be saved
        #[arg()]
        internal_path: String,

        /// List of files to insert
        #[arg(required = true)]
        files: Vec<String>,
    },

    /// Get a file or folder from the store (unencrypts it)
    #[command()]
    Get {
        /// Path to the store folder
        #[arg(long = "store", short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path in the store where it will be saved
        #[arg()]
        internal_path: String,

        /// Local path where to decrypt
        #[arg()]
        external_path: String,
    },

    /// Removes a file or folder from the store
    #[command()]
    RM {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to remove from store
        #[arg()]
        path: String,
    },

    /// List files in the store
    #[command()]
    LS {
        /// Prints sizes
        #[arg(short = 'l')]
        list: bool,

        /// Prints human-readable sizes
        #[arg(short = 'H')]
        human: bool,

        /// Path to the store folder
        #[arg(long = "store", short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path to list
        #[arg()]
        path: String,
    },

    /// Set file metadata
    #[command()]
    MetadataSet {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to set metadata
        #[arg()]
        path: String,

        /// Metadata key
        #[arg()]
        key: String,

        /// Metadata value
        #[arg()]
        value: String,
    },

    /// Get file metadata
    #[command()]
    MetadataGet {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to get metadata
        #[arg()]
        path: String,

        /// Metadata key
        #[arg()]
        key: String,
    },

    /// List file metadata
    #[command()]
    MetadataList {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to list metadata
        #[arg()]
        path: String,
    },

    /// Remove file metadata
    #[command()]
    MetadataRemove {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to get metadata
        #[arg()]
        path: String,

        /// Metadata key
        #[arg()]
        key: String,
    },

    /// Add node tag
    #[command()]
    TagAdd {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to add tag
        #[arg()]
        path: String,

        /// Tag name
        #[arg()]
        tags: Vec<String>,
    },

    /// Remove node tag
    #[command()]
    TagRemove {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to remove tag
        #[arg()]
        path: String,

        /// Tag name
        #[arg()]
        tags: Vec<String>,
    },

    /// Get node tags
    #[command()]
    TagGet {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to get tag
        #[arg()]
        path: String,
    },

    /// Get node tags
    #[command()]
    TagClear {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Path of file or folder to clear tags
        #[arg()]
        path: String,
    },

    /// List tags in the filesystem
    #[command()]
    TagList {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,
    },

    /// List nodes with tags
    #[command()]
    TagSearch {
        /// Path to the store folder
        #[arg(short = 's', env = "VOID_STORE")]
        store_path: String,

        /// Tags to search for. tag1 !tag2 will match files that contains tag1 but not tag2
        #[arg()]
        tags: Vec<String>,
    },
}

static LONG_ABOUT: &str = "
Void is an encrypted file store.

Its goal is to provide a filesystem-like way of storing encrypted files. You
can add (encrypt), get (unencrypt) and manage (list, search, remove and move)
files and folders. It also allows to set/get store-only metadata.";

#[derive(Debug, Parser)]
#[command(author, version, about = LONG_ABOUT)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Commands,

    /// Password
    #[arg(global = true, long = "password", short = 'p', env = "VOID_PSWD")]
    pub password: Option<String>,
}
