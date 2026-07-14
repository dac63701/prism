use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Tls, TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::time::Duration;

use crate::config::Config;

fn verification_email_html(username: &str, verify_url: &str, verification_code: &str, site_url: &str) -> String {
    let logo_url = format!("{}/brand/logo.svg", site_url.trim_end_matches('/'));
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Verify your Prism account</title>
</head>
<body style="margin:0;padding:0;background-color:#050816;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Oxygen,Ubuntu,sans-serif;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#050816;">
    <tr>
      <td align="center" style="padding:48px 16px;">
        <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="max-width:480px;width:100%;">
          <tr>
            <td align="center" style="padding-bottom:32px;">
              <table role="presentation" cellpadding="0" cellspacing="0">
                <tr>
                  <td style="vertical-align:middle;padding-right:10px;">
                    <img src="{logo_url}" width="36" height="36" alt="Prism" style="display:block;width:36px;height:36px;">
                  </td>
                  <td style="vertical-align:middle;">
                    <span style="font-size:22px;font-weight:700;letter-spacing:-0.02em;color:#e5eefc;">PRISM</span>
                  </td>
                </tr>
              </table>
            </td>
          </tr>

          <tr>
            <td style="background-color:#0b1222;border:1px solid #1f2a44;border-radius:24px;padding:40px 32px;">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
                <tr>
                  <td align="center" style="padding-bottom:8px;">
                    <span style="font-size:11px;text-transform:uppercase;letter-spacing:0.3em;color:#77a8ff;">Verify your email</span>
                  </td>
                </tr>
                <tr>
                  <td align="center" style="padding-bottom:16px;">
                    <h1 style="margin:0;font-size:26px;font-weight:600;letter-spacing:-0.02em;color:#e5eefc;line-height:1.3;">
                      Welcome to Prism
                    </h1>
                  </td>
                </tr>
                <tr>
                  <td align="center" style="padding-bottom:12px;">
                    <p style="margin:0;font-size:15px;line-height:1.6;color:#a1a1aa;">
                      Hi <strong style="color:#e5eefc;">{username}</strong>,<br>
                      Click the button below to verify your email address and start saving your best moments.
                    </p>
                  </td>
                </tr>

                <tr>
                  <td align="center" style="padding-bottom:24px;">
                    <table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 auto;">
                      <tr>
                        <td style="background:#10192e;border:1px solid #1f2a44;border-radius:16px;padding:20px 32px;">
                          <p style="margin:0 0 8px 0;font-size:12px;text-transform:uppercase;letter-spacing:0.2em;color:#77a8ff;">Verification code</p>
                          <p style="margin:0;font-size:36px;font-weight:700;letter-spacing:0.15em;color:#e5eefc;font-family:monospace;">{verification_code}</p>
                        </td>
                      </tr>
                    </table>
                  </td>
                </tr>

                <tr>
                  <td align="center" style="padding-bottom:24px;">
                    <table role="presentation" cellpadding="0" cellspacing="0">
                      <tr>
                        <td style="border-radius:12px;background:#4f8cff;padding:0;">
                          <a href="{verify_url}" style="display:inline-block;padding:14px 40px;border-radius:12px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;text-align:center;">
                            Verify Email Address
                          </a>
                        </td>
                      </tr>
                    </table>
                  </td>
                </tr>

                <tr>
                  <td align="center" style="padding-bottom:20px;">
                    <p style="margin:0;font-size:13px;line-height:1.5;color:#71717a;">
                      Or copy this link into your browser:<br>
                      <a href="{verify_url}" style="color:#77a8ff;text-decoration:underline;word-break:break-all;">{verify_url}</a>
                    </p>
                  </td>
                </tr>

                <tr>
                  <td style="border-top:1px solid #1f2a44;padding-top:20px;">
                    <p style="margin:0;font-size:13px;line-height:1.5;color:#52525b;">
                      Enter the 6-digit code on the sign-in page to verify instantly. This code expires in 24 hours. If you did not create an account on Prism, you can safely ignore this email.
                    </p>
                  </td>
                </tr>
              </table>
            </td>
          </tr>

          <!-- Footer -->
          <tr>
            <td align="center" style="padding-top:24px;">
              <p style="margin:0;font-size:12px;color:#3f3f46;">
                &copy; {year} Prism &middot; <a href="{site_url}" style="color:#52525b;text-decoration:none;">{site_url}</a>
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"##,
        username = username,
        verify_url = verify_url,
        verification_code = verification_code,
        logo_url = logo_url,
        site_url = site_url,
        year = chrono::Utc::now().format("%Y"),
    )
}

fn build_verification_email(
    config: &Config,
    to_email: &str,
    to_name: &str,
    token: &str,
    code: &str,
) -> Result<Message, String> {
    let verify_url = format!(
        "{}/verify-email?token={}",
        config.site_url.trim_end_matches('/'),
        token
    );
    let html = verification_email_html(to_name, &verify_url, code, &config.site_url);

    Message::builder()
        .from(
            format!("{} <{}>", config.smtp_from_name, config.smtp_from_address)
                .parse()
                .map_err(|e| format!("Invalid from address: {e}"))?,
        )
        .to(format!("{to_name} <{to_email}>")
            .parse()
            .map_err(|e| format!("Invalid to address: {e}"))?)
        .subject("Verify your Prism account")
        .header(ContentType::TEXT_HTML)
        .body(html)
        .map_err(|e| format!("Failed to build email: {e}"))
}

pub async fn send_verification_email(
    config: &Config,
    to_email: &str,
    to_name: &str,
    token: &str,
    code: &str,
) -> Result<(), String> {
    if config.smtp_host.is_empty() {
        tracing::error!(to = %to_email, "SMTP not configured — verification email not sent");
        return Err("SMTP is not configured. Set SMTP_HOST, SMTP_USERNAME, and SMTP_PASSWORD.".into());
    }

    let tls_parameters = TlsParameters::new(config.smtp_host.clone())
        .map_err(|e| format!("TLS parameters error: {e}"))?;

    let tls = match config.smtp_port {
        465 => Tls::Wrapper(tls_parameters),
        _ => Tls::Required(tls_parameters),
    };

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
            .port(config.smtp_port)
            .tls(tls)
            .credentials(creds)
            .build();

    for attempt in 1..=2 {
        let email = build_verification_email(config, to_email, to_name, token, code)?;
        match mailer.send(email).await {
            Ok(_) => {
                tracing::info!(to = %to_email, attempt, "verification email sent");
                return Ok(());
            }
            Err(e) => {
                if attempt == 2 {
                    return Err(format!("Failed to send email after two attempts: {e}"));
                }
                tracing::warn!(to = %to_email, attempt, error = %e, "verification email delivery failed; retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    unreachable!("the verification email retry loop always returns")
}

#[cfg(test)]
mod tests {
    use super::verification_email_html;

    #[test]
    fn verification_email_uses_the_canonical_logo_and_brand_colors() {
        let html = verification_email_html(
            "Prism User",
            "https://goprism.studio/verify-email?token=test",
            "123456",
            "https://goprism.studio/",
        );

        assert!(html.contains("https://goprism.studio/brand/logo.svg"));
        assert!(html.contains("#050816"));
        assert!(html.contains("#4f8cff"));
        assert!(html.contains("#77a8ff"));
        assert!(html.contains("123456"));
    }
}
