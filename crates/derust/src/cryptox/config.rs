//! Key sources (env var, file, AWS Secrets Manager) and serde-friendly configuration.
//!
//! Config structs never hold PEM: they only say where to read it from. Errors name no
//! secret content. For lists of verifying keys (`JwtVerifierConfig::keys`) use a file or
//! AWS Secrets Manager source for the configuration itself: the `config` crate used by
//! `envx::load_app_config` does not read lists by index from environment variables
//! (`KEYS__0__KID` does not work). Single-valued settings such as `TokenIssuerConfig` do
//! work with `APP__...` variables.

use crate::cryptox::error::{ConfigError, KeyError};
use crate::cryptox::jwt::{JwtVerifier, TokenIssuer, TokenIssuerSettings, TokenType};
use crate::cryptox::keys::{SigningKey, VerifyingKey, VerifyingKeySet};
use async_trait::async_trait;
use serde::Deserialize;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroizing;

/// Where a PEM comes from. The returned PEM is zeroized on drop.
#[async_trait]
pub trait KeySource: Send + Sync {
    async fn load_pem(&self) -> Result<Zeroizing<String>, KeyError>;
}

/// Reads the PEM from an environment variable (literal `\n` becomes a line break).
pub struct EnvKeySource {
    var: String,
}

impl EnvKeySource {
    pub fn new(var: impl Into<String>) -> Self {
        Self { var: var.into() }
    }
}

impl fmt::Debug for EnvKeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EnvKeySource(***)")
    }
}

#[async_trait]
impl KeySource for EnvKeySource {
    async fn load_pem(&self) -> Result<Zeroizing<String>, KeyError> {
        let raw = Zeroizing::new(std::env::var(&self.var).map_err(|_| KeyError::Source)?);
        if raw.trim().is_empty() {
            return Err(KeyError::Source);
        }
        Ok(Zeroizing::new(raw.replace("\\n", "\n")))
    }
}

/// Reads the PEM from a file.
pub struct FileKeySource {
    path: PathBuf,
}

impl FileKeySource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl fmt::Debug for FileKeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FileKeySource(***)")
    }
}

#[async_trait]
impl KeySource for FileKeySource {
    async fn load_pem(&self) -> Result<Zeroizing<String>, KeyError> {
        tokio::fs::read_to_string(&self.path)
            .await
            .map(Zeroizing::new)
            .map_err(|_| KeyError::Io)
    }
}

/// Reads the PEM from the string value of an AWS Secrets Manager secret.
/// The real call is not covered by unit tests; validate manually with localstack.
#[cfg(feature = "aws")]
pub struct SecretsManagerKeySource {
    client: crate::awsx::SecretsManagerClient,
    secret_id: String,
}

#[cfg(feature = "aws")]
impl SecretsManagerKeySource {
    pub fn new(client: crate::awsx::SecretsManagerClient, secret_id: impl Into<String>) -> Self {
        Self {
            client,
            secret_id: secret_id.into(),
        }
    }
}

#[cfg(feature = "aws")]
impl fmt::Debug for SecretsManagerKeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretsManagerKeySource(***)")
    }
}

#[cfg(feature = "aws")]
#[async_trait]
impl KeySource for SecretsManagerKeySource {
    async fn load_pem(&self) -> Result<Zeroizing<String>, KeyError> {
        let output = self
            .client
            .client
            .get_secret_value()
            .secret_id(&self.secret_id)
            .send()
            .await
            .map_err(|_| KeyError::Source)?;
        output
            .secret_string
            .map(Zeroizing::new)
            .ok_or(KeyError::Source)
    }
}

/// Serde description of a key source (no PEM inside).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum KeySourceConfig {
    Env {
        var: String,
    },
    File {
        path: PathBuf,
    },
    #[cfg(feature = "aws")]
    SecretsManager {
        secret_id: String,
    },
}

/// Turns a [`KeySourceConfig`] into a [`KeySource`]. Holds the optional AWS client.
#[derive(Clone, Default)]
pub struct KeySourceResolver {
    #[cfg(feature = "aws")]
    secrets_manager: Option<crate::awsx::SecretsManagerClient>,
}

impl fmt::Debug for KeySourceResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeySourceResolver(***)")
    }
}

impl KeySourceResolver {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "aws")]
    pub fn with_secrets_manager(mut self, client: crate::awsx::SecretsManagerClient) -> Self {
        self.secrets_manager = Some(client);
        self
    }

    /// `secrets_manager` without a configured client fails with `KeyError::Source`.
    pub fn resolve(&self, config: &KeySourceConfig) -> Result<Box<dyn KeySource>, KeyError> {
        match config {
            KeySourceConfig::Env { var } => Ok(Box::new(EnvKeySource::new(var.clone()))),
            KeySourceConfig::File { path } => Ok(Box::new(FileKeySource::new(path.clone()))),
            #[cfg(feature = "aws")]
            KeySourceConfig::SecretsManager { secret_id } => {
                let client = self.secrets_manager.clone().ok_or(KeyError::Source)?;
                Ok(Box::new(SecretsManagerKeySource::new(
                    client,
                    secret_id.clone(),
                )))
            }
        }
    }

    async fn load(&self, config: &KeySourceConfig) -> Result<Zeroizing<String>, KeyError> {
        self.resolve(config)?.load_pem().await
    }
}

/// A private key (PKCS#8 PEM) to load.
#[derive(Clone, Debug, Deserialize)]
pub struct SigningKeyConfig {
    pub kid: String,
    pub key: KeySourceConfig,
}

impl SigningKeyConfig {
    pub async fn load(&self, resolver: &KeySourceResolver) -> Result<SigningKey, KeyError> {
        let pem = resolver.load(&self.key).await?;
        SigningKey::from_pem(pem.as_bytes(), self.kid.clone())
    }
}

/// A public key (SPKI PEM) to load.
#[derive(Clone, Debug, Deserialize)]
pub struct VerifyingKeyConfig {
    pub kid: String,
    pub key: KeySourceConfig,
}

impl VerifyingKeyConfig {
    pub async fn load(&self, resolver: &KeySourceResolver) -> Result<VerifyingKey, KeyError> {
        let pem = resolver.load(&self.key).await?;
        VerifyingKey::from_pem(pem.as_bytes(), self.kid.clone())
    }
}

/// Configuration of a [`JwtVerifier`]. `issuer` and `audience` are required.
#[derive(Clone, Debug, Deserialize)]
pub struct JwtVerifierConfig {
    pub issuer: String,
    pub audience: String,
    #[serde(default)]
    pub leeway_secs: Option<u64>,
    #[serde(default)]
    pub keys: Vec<VerifyingKeyConfig>,
}

impl JwtVerifierConfig {
    pub async fn build(
        &self,
        resolver: &KeySourceResolver,
        token_type: TokenType,
    ) -> Result<JwtVerifier, ConfigError> {
        let mut keys = Vec::with_capacity(self.keys.len());
        for key in &self.keys {
            keys.push(key.load(resolver).await?);
        }
        let set = VerifyingKeySet::new(keys)?;
        let mut verifier =
            JwtVerifier::new(set, token_type, self.issuer.clone(), self.audience.clone())?;
        if let Some(secs) = self.leeway_secs {
            verifier = verifier.leeway(Duration::from_secs(secs));
        }
        Ok(verifier)
    }
}

/// Configuration of a [`TokenIssuer`] (one per channel). TTLs are required: no defaults.
#[derive(Clone, Debug, Deserialize)]
pub struct TokenIssuerConfig {
    pub issuer: String,
    pub audience: String,
    pub access_ttl_secs: u64,
    pub refresh_ttl_secs: u64,
    pub access_key: SigningKeyConfig,
    pub refresh_key: SigningKeyConfig,
}

impl TokenIssuerConfig {
    pub async fn build(&self, resolver: &KeySourceResolver) -> Result<TokenIssuer, ConfigError> {
        let access = self.access_key.load(resolver).await?;
        let refresh = self.refresh_key.load(resolver).await?;
        let issuer = TokenIssuer::new(
            access,
            refresh,
            TokenIssuerSettings {
                issuer: self.issuer.clone(),
                audience: self.audience.clone(),
                access_ttl: Duration::from_secs(self.access_ttl_secs),
                refresh_ttl: Duration::from_secs(self.refresh_ttl_secs),
            },
        )?;
        Ok(issuer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::error::JwtError;
    use crate::cryptox::keys::generate_ec_p256_keypair;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// File in the temp dir removed on drop.
    struct TempFile(PathBuf);
    impl TempFile {
        fn new(content: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "cryptox-config-test-{}-{n}.pem",
                std::process::id()
            ));
            std::fs::write(&path, content).unwrap();
            Self(path)
        }
        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn file_cfg(kid: &str, f: &TempFile) -> Value {
        json!({"kid": kid, "key": {"source": "file", "path": f.path()}})
    }

    #[tokio::test]
    async fn env_source_reads_and_converts_newlines() {
        std::env::set_var("CRYPTOX_T_ENV_OK", "line1\\nline2");
        let pem = EnvKeySource::new("CRYPTOX_T_ENV_OK")
            .load_pem()
            .await
            .unwrap();
        assert_eq!(pem.as_str(), "line1\nline2");
    }

    #[tokio::test]
    async fn env_source_fails_when_absent_or_empty() {
        std::env::set_var("CRYPTOX_T_ENV_EMPTY", "  ");
        assert_eq!(
            EnvKeySource::new("CRYPTOX_T_ENV_MISSING")
                .load_pem()
                .await
                .unwrap_err(),
            KeyError::Source
        );
        assert_eq!(
            EnvKeySource::new("CRYPTOX_T_ENV_EMPTY")
                .load_pem()
                .await
                .unwrap_err(),
            KeyError::Source
        );
    }

    #[tokio::test]
    async fn file_source_reads_and_reports_missing_file() {
        let f = TempFile::new("SECRET-CONTENT");
        let pem = FileKeySource::new(f.path()).load_pem().await.unwrap();
        assert_eq!(pem.as_str(), "SECRET-CONTENT");
        let err = FileKeySource::new("/nonexistent/cryptox/key.pem")
            .load_pem()
            .await
            .unwrap_err();
        assert_eq!(err, KeyError::Io);
        assert!(!format!("{err:?} {err}").contains("nonexistent"));
    }

    #[test]
    fn sources_debug_is_redacted() {
        assert_eq!(
            format!("{:?}", EnvKeySource::new("SECRET_VAR")),
            "EnvKeySource(***)"
        );
        assert_eq!(
            format!("{:?}", FileKeySource::new("/secret/path")),
            "FileKeySource(***)"
        );
        assert_eq!(
            format!("{:?}", KeySourceResolver::new()),
            "KeySourceResolver(***)"
        );
    }

    #[test]
    fn key_source_config_variants_deserialize() {
        let env: KeySourceConfig =
            serde_json::from_value(json!({"source": "env", "var": "X"})).unwrap();
        assert_eq!(env, KeySourceConfig::Env { var: "X".into() });
        let file: KeySourceConfig =
            serde_json::from_value(json!({"source": "file", "path": "/a/b"})).unwrap();
        assert_eq!(
            file,
            KeySourceConfig::File {
                path: "/a/b".into()
            }
        );
        assert!(serde_json::from_value::<KeySourceConfig>(json!({"source": "other"})).is_err());
        #[cfg(feature = "aws")]
        {
            let sm: KeySourceConfig =
                serde_json::from_value(json!({"source": "secrets_manager", "secret_id": "s"}))
                    .unwrap();
            assert_eq!(
                sm,
                KeySourceConfig::SecretsManager {
                    secret_id: "s".into()
                }
            );
        }
    }

    #[cfg(feature = "aws")]
    #[tokio::test]
    async fn secrets_manager_without_client_fails_and_with_client_builds() {
        let cfg = KeySourceConfig::SecretsManager {
            secret_id: "s".into(),
        };
        assert_eq!(
            KeySourceResolver::new().resolve(&cfg).err(),
            Some(KeyError::Source)
        );
        let conf = aws_sdk_secretsmanager::Config::builder()
            .behavior_version_latest()
            .build();
        let client = crate::awsx::SecretsManagerClient {
            client: aws_sdk_secretsmanager::Client::from_conf(conf),
        };
        let source = SecretsManagerKeySource::new(client.clone(), "secret");
        assert_eq!(format!("{source:?}"), "SecretsManagerKeySource(***)");
        assert!(KeySourceResolver::new()
            .with_secrets_manager(client)
            .resolve(&cfg)
            .is_ok());
    }

    async fn issuer_cfg(access: &TempFile, refresh: &TempFile, extra: Value) -> TokenIssuerConfig {
        let mut v = json!({
            "issuer": "id-web", "audience": "bff-web",
            "access_ttl_secs": 300, "refresh_ttl_secs": 3600,
            "access_key": file_cfg("a1", access), "refresh_key": file_cfg("r1", refresh),
        });
        for (k, val) in extra.as_object().unwrap() {
            v[k] = val.clone();
        }
        serde_json::from_value(v).unwrap()
    }

    #[tokio::test]
    async fn issuer_and_verifier_end_to_end_from_files() {
        let (apriv, apub) = generate_ec_p256_keypair().unwrap();
        let (rpriv, rpub) = generate_ec_p256_keypair().unwrap();
        let (fa, fr) = (TempFile::new(&apriv), TempFile::new(&rpriv));
        let (pa, pr) = (TempFile::new(&apub), TempFile::new(&rpub));
        let resolver = KeySourceResolver::new();
        let issuer = issuer_cfg(&fa, &fr, json!({}))
            .await
            .build(&resolver)
            .await
            .unwrap();

        let vcfg: JwtVerifierConfig = serde_json::from_value(json!({
            "issuer": "id-web", "audience": "bff-web", "leeway_secs": 5,
            "keys": [file_cfg("a1", &pa)],
        }))
        .unwrap();
        let verifier = vcfg.build(&resolver, TokenType::Access).await.unwrap();
        let token = issuer.access_signer().sign("u1", ()).unwrap();
        let ok: crate::cryptox::VerifiedJwt<Value> = verifier.verify(&token).unwrap();
        assert_eq!(ok.subject(), "u1");

        let rcfg: JwtVerifierConfig = serde_json::from_value(json!({
            "issuer": "id-web", "audience": "bff-web", "keys": [file_cfg("r1", &pr)],
        }))
        .unwrap();
        let refresh_v = rcfg.build(&resolver, TokenType::Refresh).await.unwrap();
        let rt = issuer.refresh_signer().sign("u1", ()).unwrap();
        assert!(refresh_v.verify::<Value>(&rt).is_ok());
        assert!(refresh_v.verify::<Value>(&token).is_err());
    }

    #[tokio::test]
    async fn issuer_with_same_file_for_both_keys_fails() {
        let (priv_pem, _) = generate_ec_p256_keypair().unwrap();
        let f = TempFile::new(&priv_pem);
        let err = issuer_cfg(&f, &f, json!({}))
            .await
            .build(&KeySourceResolver::new())
            .await
            .unwrap_err();
        assert_eq!(err, ConfigError::Jwt(JwtError::InvalidConfig));
    }

    #[tokio::test]
    async fn empty_issuer_or_audience_and_missing_fields() {
        let (a, _) = generate_ec_p256_keypair().unwrap();
        let (r, _) = generate_ec_p256_keypair().unwrap();
        let (fa, fr) = (TempFile::new(&a), TempFile::new(&r));
        let resolver = KeySourceResolver::new();
        for extra in [json!({"issuer": ""}), json!({"audience": ""})] {
            let err = issuer_cfg(&fa, &fr, extra)
                .await
                .build(&resolver)
                .await
                .unwrap_err();
            assert_eq!(err, ConfigError::Jwt(JwtError::InvalidConfig));
        }
        assert!(
            serde_json::from_value::<JwtVerifierConfig>(json!({"audience": "a", "keys": []}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<JwtVerifierConfig>(json!({"issuer": "i", "keys": []}))
                .is_err()
        );
        assert!(serde_json::from_value::<TokenIssuerConfig>(
            json!({"issuer": "i", "audience": "a"})
        )
        .is_err());
        let empty: JwtVerifierConfig =
            serde_json::from_value(json!({"issuer": "i", "audience": "a"})).unwrap();
        assert_eq!(
            empty.build(&resolver, TokenType::Access).await.unwrap_err(),
            ConfigError::Jwt(JwtError::InvalidConfig)
        );
    }

    #[tokio::test]
    async fn wrong_key_format_fails_without_leaking_content() {
        let (_, public) = generate_ec_p256_keypair().unwrap();
        let body = public.lines().nth(1).unwrap().to_owned();
        let f = TempFile::new(&public);
        let cfg: SigningKeyConfig = serde_json::from_value(file_cfg("k", &f)).unwrap();
        let err = cfg.load(&KeySourceResolver::new()).await.unwrap_err();
        assert!(matches!(err, KeyError::InvalidPem | KeyError::NotPkcs8));
        assert!(!format!("{err:?} {err}").contains(&body));
        assert!(!format!("{cfg:?}").contains(&body));
    }

    #[tokio::test]
    async fn env_key_source_config_loads_a_key() {
        let (private, _) = generate_ec_p256_keypair().unwrap();
        std::env::set_var("CRYPTOX_T_KEY_PEM", private.replace('\n', "\\n"));
        let cfg: SigningKeyConfig = serde_json::from_value(
            json!({"kid": "k", "key": {"source": "env", "var": "CRYPTOX_T_KEY_PEM"}}),
        )
        .unwrap();
        assert_eq!(
            cfg.load(&KeySourceResolver::new()).await.unwrap().kid(),
            "k"
        );
    }

    #[cfg(feature = "env")]
    #[tokio::test]
    async fn token_issuer_config_loads_through_envx() {
        let (a, _) = generate_ec_p256_keypair().unwrap();
        let (r, _) = generate_ec_p256_keypair().unwrap();
        let (fa, fr) = (TempFile::new(&a), TempFile::new(&r));
        for (k, v) in [
            ("ISSUER", "id-web"),
            ("AUDIENCE", "bff-web"),
            ("ACCESS_TTL_SECS", "300"),
            ("REFRESH_TTL_SECS", "3600"),
            ("ACCESS_KEY__KID", "a1"),
            ("ACCESS_KEY__KEY__SOURCE", "file"),
            ("ACCESS_KEY__KEY__PATH", fa.path()),
            ("REFRESH_KEY__KID", "r1"),
            ("REFRESH_KEY__KEY__SOURCE", "file"),
            ("REFRESH_KEY__KEY__PATH", fr.path()),
        ] {
            std::env::set_var(format!("CRYPTOXT__{k}"), v);
        }
        let cfg: TokenIssuerConfig = {
            #[cfg(feature = "env_from_secrets_manager")]
            let loaded = crate::envx::load_app_config(
                crate::envx::Environment::Test,
                Some("CRYPTOXT"),
                vec![],
            )
            .await;
            #[cfg(not(feature = "env_from_secrets_manager"))]
            let loaded =
                crate::envx::load_app_config(crate::envx::Environment::Test, Some("CRYPTOXT"))
                    .await;
            loaded.unwrap()
        };
        assert_eq!(cfg.access_ttl_secs, 300);
        assert!(cfg.build(&KeySourceResolver::new()).await.is_ok());
    }
}
