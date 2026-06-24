//! Email portal — compose an email in the default mail client.

use anyhow::Result;
use ashpd::desktop::email::EmailRequest;

pub struct EmailPortal;

impl EmailPortal {
    /// Open the default mail client with a pre-composed email.
    pub async fn compose_email(
        address: Option<&str>,
        subject: Option<&str>,
        body: Option<&str>,
        cc: &[String],
        bcc: &[String],
    ) -> Result<()> {
        let mut req = EmailRequest::default();

        if let Some(a) = address {
            req = req.address(a);
        }
        if let Some(s) = subject {
            req = req.subject(s);
        }
        if let Some(b) = body {
            req = req.body(b);
        }
        if !cc.is_empty() {
            req = req.cc(cc.iter().map(|s| s.as_str()));
        }
        if !bcc.is_empty() {
            req = req.bcc(bcc.iter().map(|s| s.as_str()));
        }

        req.send().await?.response()?;
        Ok(())
    }
}
