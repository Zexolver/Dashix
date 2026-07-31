use serde::{Deserialize, Serialize};

const BASE: &str = "http://127.0.0.1:7878";

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InterfaceDto {
    pub name: String,
    pub mac: Option<String>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteTargetDto {
    Static { path: String, hot_reload: bool },
    Port { port: u16 },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteDto {
    pub id: String,
    pub subdomain: String,
    pub target: RouteTargetDto,
    pub tls: bool,
    #[allow(dead_code)]
    pub internal_port: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct NewRouteDto {
    pub subdomain: String,
    pub target: RouteTargetDto,
    pub tls: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Dynv6ConfigDto {
    pub enabled: bool,
    pub token: Option<String>,
    pub interface: Option<String>,
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dynv6StatusDto {
    pub domain: String,
    #[allow(dead_code)]
    pub last_attempt: Option<String>,
    pub last_success: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct L4RuleDto {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub listen_port: u16,
    pub upstream_port: u16,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SecurityConfigDto {
    pub l4_rules: Vec<L4RuleDto>,
    pub blocked_ips: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MailAccountDto {
    pub address: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MailConfigDto {
    pub domain: Option<String>,
    pub accounts: Vec<MailAccountDto>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn list_interfaces(&self) -> anyhow::Result<Vec<InterfaceDto>> {
        #[derive(Deserialize)]
        struct Resp {
            interfaces: Vec<InterfaceDto>,
        }
        let resp: Resp = self
            .http
            .get(format!("{BASE}/api/network/interfaces"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.interfaces)
    }

    pub async fn get_dynv6(&self) -> anyhow::Result<Dynv6ConfigDto> {
        Ok(self
            .http
            .get(format!("{BASE}/api/dynv6"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn set_dynv6(&self, cfg: &Dynv6ConfigDto) -> anyhow::Result<()> {
        self.http
            .put(format!("{BASE}/api/dynv6"))
            .json(cfg)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn dynv6_status(&self) -> anyhow::Result<Vec<Dynv6StatusDto>> {
        #[derive(Deserialize)]
        struct Resp {
            status: Vec<Dynv6StatusDto>,
        }
        let resp: Resp = self
            .http
            .get(format!("{BASE}/api/dynv6/status"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.status)
    }

    pub async fn sync_dynv6_now(&self) -> anyhow::Result<Vec<Dynv6StatusDto>> {
        #[derive(Deserialize)]
        struct Resp {
            status: Vec<Dynv6StatusDto>,
        }
        let resp: Resp = self
            .http
            .post(format!("{BASE}/api/dynv6/sync"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.status)
    }

    pub async fn list_routes(&self) -> anyhow::Result<Vec<RouteDto>> {
        Ok(self
            .http
            .get(format!("{BASE}/api/routes"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn create_route(&self, req: &NewRouteDto) -> anyhow::Result<RouteDto> {
        Ok(self
            .http
            .post(format!("{BASE}/api/routes"))
            .json(req)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn delete_route(&self, id: &str) -> anyhow::Result<()> {
        self.http
            .delete(format!("{BASE}/api/routes/{id}"))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn apply(&self) -> anyhow::Result<()> {
        self.http
            .post(format!("{BASE}/api/apply"))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_security(&self) -> anyhow::Result<SecurityConfigDto> {
        Ok(self
            .http
            .get(format!("{BASE}/api/security"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn set_security(&self, cfg: &SecurityConfigDto) -> anyhow::Result<()> {
        self.http
            .put(format!("{BASE}/api/security"))
            .json(cfg)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_mail(&self) -> anyhow::Result<MailConfigDto> {
        Ok(self
            .http
            .get(format!("{BASE}/api/mail"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn set_mail(&self, cfg: &MailConfigDto) -> anyhow::Result<()> {
        self.http
            .put(format!("{BASE}/api/mail"))
            .json(cfg)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
