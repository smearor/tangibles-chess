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
