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
}

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
        format!("{:.1} GB", self.bytes as f64 / 1e9)
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
