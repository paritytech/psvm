use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::versions::get_polkadot_sdk_versions;

/// The structure to hold the cached list of versions
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Cache {
    /// Data to be cached
    pub data: Vec<String>,
}

impl Cache {
    // Load cache from a file
    pub fn load(path: &PathBuf) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let cache: Cache = serde_json::from_str(&contents)?;
        Ok(cache)
    }

    // Save cache to a file
    pub fn save(&self, path: &PathBuf) -> io::Result<()> {
        let contents = serde_json::to_string(&self)?;
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }
}

/// Retrieves the list of Polkadot SDK versions, either from a local cache or by fetching them anew.
///
/// This function first attempts to load the list of Polkadot SDK versions from a local cache file.
/// If the cache file exists and can be loaded, the cached data is returned. If the cache does not exist,
/// is unreadable, or any other error occurs during loading, the function logs an error message,
/// fetches the list of versions by calling `get_polkadot_sdk_versions`, caches the newly fetched list,
/// and then returns it.
///
/// # Returns
/// A `Result` wrapping a vector of strings, where each string is a version of the Polkadot SDK.
/// If the operation is successful, `Ok(Vec<String>)` is returned, containing the list of versions.
/// If an error occurs during fetching new versions or saving them to the cache, an error is returned
/// wrapped in `Err(Box<dyn std::error::Error>)`.
///
/// # Errors
/// This function can return an error in several cases, including but not limited to:
/// - Failure to read the cache file due to permissions or file not found.
/// - Failure to write to the cache file, possibly due to permissions issues.
/// - Errors returned by `get_polkadot_sdk_versions` during the fetching process.
///
/// # Examples
/// ```
/// #[tokio::main]
/// async fn main() {
///     match get_polkadot_sdk_versions_from_cache().await {
///         Ok(versions) => println!("Polkadot SDK Versions: {:?}", versions),
///         Err(e) => eprintln!("Failed to get Polkadot SDK versions: {}", e),
///     }
/// }
/// ```
pub async fn get_polkadot_sdk_versions_from_cache() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Path to the cache file. should save as a constant once path is finalized
    let cache_path = PathBuf::from("./cache.json");

    // Attempt to load the cache
    let cache = Cache::load(&cache_path);

    let data = if let Ok(cache) = cache {
        cache.data
    } else {
        log::error!("Cache file doesn't exist or failed to load, fetching new data");
        let new_data = get_polkadot_sdk_versions().await?;
        let new_cache = Cache {
            data: new_data.clone(),
        };
        new_cache.save(&cache_path)?;
        new_data
    };

    Ok(data)
}
