use anyhow::anyhow;
use k8s_openapi::api::core::v1::Secret;
use secrecy::SecretString;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct DockerConfigJson {
    pub auths: HashMap<String, DockerConfigJsonAuth>,
}

#[derive(Deserialize, Debug)]
pub struct DockerConfigJsonAuth {
    pub username: Option<String>,
    pub password: Option<SecretString>,
    pub auth: Option<SecretString>,
}

impl DockerConfigJson {
    pub fn from_secret(secret: Secret) -> Result<Self, anyhow::Error> {
        if let Some(data) = secret.data.clone() {
            let bytes = data
                .get(".dockerconfigjson")
                .ok_or(anyhow!("No .dockerconfigjson in secret"))?;
            let b = bytes.clone();

            let res: Self = match std::str::from_utf8(&b.0) {
                Ok(s) => Ok(serde_json::from_str(s)?),
                Err(e) => Err(anyhow!("Error decoding secret: {}", e)),
            }?;

            if res
                .auths
                .values()
                .any(|a| !(a.username.is_some() && a.password.is_some() || a.auth.is_some()))
            {
                Err(anyhow!(
                    "Registry configuration requires either auth or username and password fields"
                ))
            } else {
                Ok(res)
            }
        } else {
            Err(anyhow!("No data in secret"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;
    use serde_json::json;

    use secrecy::ExposeSecret;
    #[test]
    fn test_docker_config() {
        // { "auths": { "https://index.docker.io/v1/": { "auth": "c3R...zE2" }, "https://other.docker.io/v1/": { "username": "user", "password": "supersecret" } } }
        let secret: Secret = serde_json::from_value(json!({
            "metadata": {
                "name": "test",
                "namespace": "test"
            },
            "data": {
                ".dockerconfigjson": "eyAiYXV0aHMiOiB7ICJodHRwczovL2luZGV4LmRvY2tlci5pby92MS8iOiB7ICJhdXRoIjogImMzUi4uLnpFMiIgfSwgImh0dHBzOi8vb3RoZXIuZG9ja2VyLmlvL3YxLyI6IHsgInVzZXJuYW1lIjogInVzZXIiLCAicGFzc3dvcmQiOiAic3VwZXJzZWNyZXQiIH0gfSB9Cg=="
            }
        }))
        .unwrap();

        let config = DockerConfigJson::from_secret(secret).unwrap();
        assert_eq!(
            config
                .auths
                .get("https://other.docker.io/v1/")
                .map(|a| a.username.clone()),
            Some(Some("user".into()))
        );
        assert_eq!(
            config
                .auths
                .get("https://index.docker.io/v1/")
                .map(|a| a.auth.as_ref().map(|s| s.expose_secret()).clone()),
            Some(Some(&"c3R...zE2".to_string()))
        );
    }
}
