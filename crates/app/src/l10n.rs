use fluent::FluentArgs;
use fluent::FluentBundle;
use fluent::FluentResource;
use fluent_langneg::NegotiationStrategy;
use fluent_langneg::negotiate_languages;
use unic_langid::LanguageIdentifier;

/// Compile-time embedded Fluent bundles.
const RESOURCES: &[(&str, &str)] = &[
    ("en", include_str!("../../../i18n/en/main.ftl")),
    ("de", include_str!("../../../i18n/de/main.ftl")),
];

/// Localization wrapper: negotiates the UI language from `LANGUAGE`/`LANG`
/// once, then serves message lookups with English fallback.
pub struct L10n {
    /// Active Fluent bundle (negotiated locale).
    bundle: FluentBundle<FluentResource>,
}

impl L10n {
    /// Negotiates from the process locale environment.
    pub fn system() -> Self {
        let requested: Vec<LanguageIdentifier> = std::env::var("LANGUAGE")
            .ok()
            .map(|l| l.split(':').filter_map(|l| l.parse().ok()).collect())
            .unwrap_or_default();
        let supported: Vec<LanguageIdentifier> = RESOURCES.iter().filter_map(|(code, _)| code.parse().ok()).collect();
        let fallback: LanguageIdentifier = "en".parse().unwrap_or_default();
        let negotiated = negotiate_languages(&requested, &supported, Some(&fallback), NegotiationStrategy::Filtering);
        Self::for_locale(negotiated.first().map(|l| (*l).clone()).unwrap_or(fallback))
    }

    /// A bundle for a concrete locale.
    pub fn for_locale(locale: LanguageIdentifier) -> Self {
        let mut bundle = FluentBundle::new(vec![locale.clone()]);
        let locale_code = locale.to_string();
        if let Some((_, text)) = RESOURCES.iter().find(|(code, _)| *code == locale_code)
            && let Ok(resource) = FluentResource::try_new(text.to_string())
        {
            let _ = bundle.add_resource(resource);
        }
        // English as compile-time fallback for missing keys.
        if locale_code != "en"
            && let Some((_, text)) = RESOURCES.iter().find(|(code, _)| *code == "en")
            && let Ok(resource) = FluentResource::try_new(text.to_string())
        {
            bundle.add_resource_overriding(resource);
        }
        Self { bundle }
    }

    /// Looks up a message id; falls back to the id itself when absent.
    pub fn msg(&self, id: &str) -> String {
        let mut errors = Vec::new();
        self.bundle
            .get_message(id)
            .and_then(|message| message.value())
            .map(|pattern| self.bundle.format_pattern(pattern, None, &mut errors).to_string())
            .unwrap_or_else(|| id.to_string())
    }

    /// Looks up a message id with arguments.
    pub fn msg_args(&self, id: &str, args: &FluentArgs) -> String {
        let mut errors = Vec::new();
        self.bundle
            .get_message(id)
            .and_then(|message| message.value())
            .map(|pattern| self.bundle.format_pattern(pattern, Some(args), &mut errors).to_string())
            .unwrap_or_else(|| id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::path::PathBuf;

    use super::RESOURCES;

    /// Every `l10n.msg`/`l10n.msg_args` literal id in the app sources
    /// resolves to a Fluent id in each embedded catalog. `msg` falls
    /// back to the id itself, so a typo otherwise surfaces only as an
    /// untranslated line at runtime.
    #[test]
    fn message_ids_exist_in_every_locale() {
        for key in literal_msg_ids() {
            for (locale, ftl) in RESOURCES {
                assert!(ftl_ids(ftl).contains(&key.as_str()), "`{key}` used via l10n.msg but missing in i18n/{locale}/main.ftl");
            }
        }
    }

    /// Fluent ids defined in a catalog (top-level `id =` lines).
    fn ftl_ids(ftl: &str) -> BTreeSet<&str> {
        ftl.lines()
            .filter_map(|line| line.split_once('=').map(|(id, _)| id.trim()))
            .filter(|id| !id.is_empty())
            .collect()
    }

    /// Message ids passed literally to `l10n.msg`/`l10n.msg_args`
    /// across the app sources.
    fn literal_msg_ids() -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        for file in rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
            // Skip this module: its test names the call sites and would
            // match the scan needles itself.
            if file.file_name().is_some_and(|name| name == "l10n.rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&file) else {
                continue;
            };
            for needle in ["l10n.msg(\"", "l10n.msg_args(\""] {
                for (index, _) in source.match_indices(needle) {
                    if let Some(end) = source[index + needle.len()..].find('"') {
                        ids.insert(source[index + needle.len()..index + needle.len() + end].to_string());
                    }
                }
            }
        }
        ids
    }

    /// All `.rs` files below `dir`, recursively.
    fn rust_files(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
        files
    }
}
