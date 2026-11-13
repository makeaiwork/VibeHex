//! Labs › Clean up on your own server › Built in: the language models Hex
//! downloads itself for dictation cleanup, run by the bundled `llama-server`
//! ([`super::engine`]). Each is a GGUF file pinned to a Hugging Face commit,
//! size, and SHA-256, kept in `<support>/cleanup-models/` apart from the
//! speech models. A download resumes from `<file>.partial` and only a file
//! whose size and SHA-256 match is moved into place, so a file there is a
//! verified one.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use color_eyre::eyre::{Result, WrapErr, bail, eyre};

/// One downloadable cleanup model.
#[derive(Debug, Eq, PartialEq)]
pub struct LocalModel {
    /// Saved in `dictation_processing.server.local_model`.
    pub id: &'static str,
    pub name: &'static str,
    /// Who made it, and who converted it to GGUF.
    pub publisher: &'static str,
    pub license: &'static str,
    repository: &'static str,
    revision: &'static str,
    pub filename: &'static str,
    pub bytes: u64,
    sha256: &'static str,
    /// How fast it rewrites, 1 to 5, as measured on an M-series Mac with
    /// Metal: Gemma 4 E2B about 93 tokens a second, Qwen3 4B about 61,
    /// Gemma 4 E4B about 47.
    pub speed: u8,
    /// How well it cleans up, 1 to 5: on 44 misheard vocabulary forms from
    /// real Russian dictations, E4B wrote 40 as their terms, Qwen3 37, E2B
    /// 36; Qwen3 also follows instructions more closely than E2B.
    pub quality: u8,
    /// Its attention cache's shape, from the file's GGUF header, for
    /// [`LocalModel::memory`].
    cache: Cache,
}

/// What a model's attention cache holds per token, as its GGUF header
/// says (`block_count`, `attention.shared_kv_layers`, `head_count_kv`,
/// `key_length` and `value_length`, their `_swa` forms, and
/// `sliding_window_pattern`). llama-server keeps it in f16, 2 bytes a
/// number, the way the engine starts it for dictation (no `--cache-type`).
#[derive(Debug, Eq, PartialEq)]
struct Cache {
    /// Layers that keep the whole window: their own keys and values (a
    /// layer that shares an earlier one's keeps none).
    full_layers: u64,
    /// Bytes a token takes in one of them: K and V heads times their
    /// lengths, at 2 bytes.
    full_bytes: u64,
    /// Sliding-window layers, which keep only [`WINDOW_CELLS`] tokens.
    window_layers: u64,
    window_bytes: u64,
}

/// What a sliding-window layer keeps: llama.cpp's window (512 for Gemma 4)
/// plus its 512-token batch, `n_swa + n_ubatch`.
const WINDOW_CELLS: u64 = 1024;

/// Beyond the file and the cache, what a running llama-server holds: its
/// compute buffers and the process itself. Measured with `footprint` on
/// Gemma 4 E4B serving dictation: 505 MB beside the mapped file, 176 MB
/// of it the cache, so about 0.33 GB; rounded up.
pub const OVERHEAD: u64 = 400_000_000;

/// The models offered, the recommended one first.
pub const MODELS: &[LocalModel] = &[
    LocalModel {
        id: "qwen3-4b-instruct-2507",
        name: "Qwen3 4B Instruct",
        publisher: "Qwen, GGUF by Unsloth",
        license: "Apache 2.0",
        repository: "unsloth/Qwen3-4B-Instruct-2507-GGUF",
        revision: "a06e946bb6b655725eafa393f4a9745d460374c9",
        filename: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        bytes: 2_497_281_120,
        sha256: "3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597",
        speed: 4,
        quality: 4,
        // 36 layers, 8 KV heads of 128 + 128.
        cache: Cache {
            full_layers: 36,
            full_bytes: 8 * (128 + 128) * 2,
            window_layers: 0,
            window_bytes: 0,
        },
    },
    LocalModel {
        id: "gemma-4-e4b-it",
        name: "Gemma 4 E4B",
        publisher: "Google, GGUF by Unsloth",
        license: "Apache 2.0",
        repository: "unsloth/gemma-4-E4B-it-GGUF",
        revision: "bfc15c382204943c3a8fff0c750b94ae2364d7a3",
        filename: "gemma-4-E4B-it-Q4_K_M.gguf",
        bytes: 4_977_171_584,
        sha256: "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87",
        speed: 3,
        quality: 5,
        // 42 layers, the last 18 sharing: of the 24 with a cache, 4 global
        // (2 KV heads of 512 + 512) and 20 sliding (2 of 256 + 256).
        cache: Cache {
            full_layers: 4,
            full_bytes: 2 * (512 + 512) * 2,
            window_layers: 20,
            window_bytes: 2 * (256 + 256) * 2,
        },
    },
    LocalModel {
        id: "gemma-4-e2b-it",
        name: "Gemma 4 E2B",
        publisher: "Google, GGUF by Unsloth",
        license: "Apache 2.0",
        repository: "unsloth/gemma-4-E2B-it-GGUF",
        revision: "0314792d7f1f7e229411f620751375812bb9faf2",
        filename: "gemma-4-E2B-it-Q4_K_M.gguf",
        bytes: 3_106_738_272,
        sha256: "740185b21d22ceb83a11c3aa62ad5842ef32c70f6096d756bbee85a1e4ec34b8",
        speed: 5,
        quality: 3,
        // 35 layers, the last 20 sharing: of the 15 with a cache, 3 global
        // (1 KV head of 512 + 512) and 12 sliding (1 of 256 + 256).
        cache: Cache {
            full_layers: 3,
            full_bytes: (512 + 512) * 2,
            window_layers: 12,
            window_bytes: (256 + 256) * 2,
        },
    },
];

/// The model saved as `id`, if Hex offers it.
pub fn find(id: &str) -> Option<&'static LocalModel> {
    MODELS.iter().find(|model| model.id == id.trim())
}

/// The model a saved choice runs: the saved one, else the recommended one.
pub fn chosen(id: &str) -> &'static LocalModel {
    find(id).unwrap_or(&MODELS[0])
}

/// Where downloaded cleanup models live.
pub fn dir() -> Result<PathBuf> {
    Ok(crate::app_paths::support_dir()?.join("cleanup-models"))
}

impl LocalModel {
    fn url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repository, self.revision, self.filename
        )
    }

    pub fn path(&self) -> Result<PathBuf> {
        Ok(dir()?.join(self.filename))
    }

    fn partial(&self) -> Result<PathBuf> {
        Ok(dir()?.join(format!("{}.partial", self.filename)))
    }

    /// Whether the verified file is in place.
    pub fn installed(&self) -> bool {
        self.path()
            .and_then(|path| Ok(fs::metadata(path)?))
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == self.bytes)
    }

    /// Its full name where it comes from, as LM Studio shows it:
    /// `unsloth/gemma-4-E4B-it-GGUF`.
    pub fn repository(&self) -> &'static str {
        self.repository
    }

    /// Its quantization from the file name, such as `Q4_K_M`.
    pub fn quantization(&self) -> &'static str {
        let stem = self.filename.trim_end_matches(".gguf");
        stem.rsplit_once('-')
            .map_or(stem, |(_, quantization)| quantization)
    }

    /// The bits a weight mostly has, such as 4 for `Q4_K_M`.
    pub fn bits(&self) -> Option<u32> {
        let quantization = self.quantization();
        let digits: String = quantization
            .strip_prefix(['Q', 'q'])?
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    }

    /// Size for a person, such as "2.5 GB".
    pub fn size_label(&self) -> String {
        gigabytes(self.bytes)
    }

    /// Its attention cache in the window the engine starts dictation with
    /// ([`super::engine::CONTEXT_TOKENS`]), in bytes.
    pub fn cache_bytes(&self) -> u64 {
        let cache = &self.cache;
        let window = u64::from(super::engine::CONTEXT_TOKENS);
        cache.full_layers * cache.full_bytes * window
            + cache.window_layers * cache.window_bytes * WINDOW_CELLS.min(window)
    }

    /// About how much memory it takes while it runs: the file (mapped
    /// whole, every layer on the GPU), its cache, and [`OVERHEAD`].
    pub fn memory(&self) -> u64 {
        self.bytes + self.cache_bytes() + OVERHEAD
    }

    /// Stops the engine if it runs this model, then deletes the file and any
    /// partial download.
    pub fn remove(&self) -> Result<()> {
        super::engine::stop_model(self.id);
        for path in [self.path()?, self.partial()?] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .wrap_err_with(|| format!("couldn't delete {}", path.display()));
                }
            }
        }
        Ok(())
    }
}

/// Bytes for a person, in decimal gigabytes as a download is counted:
/// "2.5 GB".
pub fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

/// Downloads `model`, resuming a partial file, and moves it into place once
/// its size and SHA-256 match. `bytes` follows the partial file's length.
pub fn download(model: &LocalModel, canceled: &AtomicBool, bytes: &AtomicU64) -> Result<PathBuf> {
    let path = model.path()?;
    if model.installed() {
        bytes.store(model.bytes, Ordering::Relaxed);
        return Ok(path);
    }
    fs::create_dir_all(dir()?)?;
    let partial = model.partial()?;
    let url = model.url();
    let user_agent = format!("hex/{}", crate::VERSION);
    crate::net::download(
        &crate::net::Download {
            url: &url,
            user_agent: &user_agent,
            size: Some(model.bytes),
            canceled: Some(canceled),
        },
        &partial,
        |done| bytes.store(done, Ordering::Relaxed),
    )?;
    promote(model, &partial, &path, canceled)?;
    Ok(path)
}

/// Checks the finished partial file and renames it into place; a file that
/// doesn't match is deleted, so the next download starts over.
fn promote(
    model: &LocalModel,
    partial: &std::path::Path,
    path: &std::path::Path,
    canceled: &AtomicBool,
) -> Result<()> {
    let size = fs::metadata(partial)?.len();
    if size != model.bytes {
        let _ = fs::remove_file(partial);
        bail!(
            "{} downloaded as {size} bytes, not {}; try again",
            model.name,
            model.bytes
        );
    }
    let digest = crate::transcription::models::sha256_hex(partial, Some(canceled))?;
    if canceled.load(Ordering::Acquire) {
        return Err(eyre!("the download was cancelled"));
    }
    if digest != model.sha256 {
        let _ = fs::remove_file(partial);
        bail!(
            "{} didn't match its checksum and was deleted; try again",
            model.name
        );
    }
    fs::rename(partial, path).wrap_err("couldn't move the downloaded model into place")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_is_pinned_and_unique() {
        let mut ids = std::collections::HashSet::new();
        for model in MODELS {
            assert!(ids.insert(model.id), "{} is listed twice", model.id);
            assert_eq!(model.revision.len(), 40, "{}", model.id);
            assert_eq!(model.sha256.len(), 64, "{}", model.id);
            assert!(model.filename.ends_with(".gguf"));
            assert_eq!(model.quantization(), "Q4_K_M");
            assert_eq!(model.bits(), Some(4));
            assert!(model.repository().contains('/'));
            assert!(model.url().starts_with("https://huggingface.co/"));
            assert!(model.bytes > 0);
        }
        assert_eq!(chosen("").id, MODELS[0].id);
        assert_eq!(chosen("not-a-model").id, MODELS[0].id);
        assert_eq!(find(MODELS[0].id), Some(&MODELS[0]));
    }

    /// What each takes while it runs, from its header's numbers: Qwen3's
    /// cache is a whole window on every layer, Gemma's mostly a sliding
    /// one on the layers that don't share.
    #[test]
    fn each_model_says_what_it_takes_while_it_runs() {
        let memory: Vec<(&str, u64, String)> = MODELS
            .iter()
            .map(|model| (model.id, model.cache_bytes(), gigabytes(model.memory())))
            .collect();
        assert_eq!(
            memory,
            [
                ("qwen3-4b-instruct-2507", 1_207_959_552, "4.1 GB".to_owned()),
                ("gemma-4-e4b-it", 176_160_768, "5.6 GB".to_owned()),
                ("gemma-4-e2b-it", 62_914_560, "3.6 GB".to_owned()),
            ]
        );
    }

    #[test]
    fn only_a_matching_file_is_moved_into_place() {
        let root = std::env::temp_dir().join(format!("hex-local-model-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let model = LocalModel {
            id: "test",
            name: "Test",
            publisher: "",
            license: "",
            repository: "",
            revision: "",
            filename: "test.gguf",
            bytes: 5,
            // SHA-256 of "hello".
            sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            speed: 3,
            quality: 3,
            cache: Cache {
                full_layers: 0,
                full_bytes: 0,
                window_layers: 0,
                window_bytes: 0,
            },
        };
        let partial = root.join("test.gguf.partial");
        let path = root.join("test.gguf");
        let canceled = AtomicBool::new(false);

        fs::write(&partial, "world").unwrap();
        let error = promote(&model, &partial, &path, &canceled).unwrap_err();
        assert!(error.to_string().contains("checksum"), "{error}");
        assert!(!partial.exists() && !path.exists());

        fs::write(&partial, "hi").unwrap();
        assert!(promote(&model, &partial, &path, &canceled).is_err());
        assert!(!partial.exists() && !path.exists());

        fs::write(&partial, "hello").unwrap();
        promote(&model, &partial, &path, &canceled).unwrap();
        assert!(!partial.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        let _ = fs::remove_dir_all(root);
    }
}
