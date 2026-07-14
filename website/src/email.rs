use lettre::{
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use lettre::message::header::ContentType;

use crate::config::Config;

fn verification_email_html(username: &str, verify_url: &str, site_url: &str) -> String {
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
          <!-- Logo -->
          <tr>
            <td align="center" style="padding-bottom:32px;">
              <table role="presentation" cellpadding="0" cellspacing="0">
                <tr>
                  <td style="vertical-align:middle;padding-right:10px;">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                      <path d="M12 2L2 7V17L12 22L22 17V7L12 2Z" fill="#6366f1" stroke="#818cf8" stroke-width="0.5"/>
                      <path d="M12 22V12M12 2V12M12 12L22 7M12 12L2 7" stroke="#a5b4fc" stroke-width="0.5" opacity="0.6"/>
                    </svg>
                  </td>
                  <td style="vertical-align:middle;">
                    <span style="font-size:22px;font-weight:700;letter-spacing:-0.02em;color:#ffffff;">PRISM</span>
                  </td>
                </tr>
              </table>
            </td>
          </tr>

          <!-- Card -->
          <tr>
            <td style="background:linear-gradient(180deg,rgba(16,25,46,0.98),rgba(8,13,26,0.98));border:1px solid #1f2a44;border-radius:24px;padding:40px 32px;">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
                <tr>
                  <td align="center" style="padding-bottom:8px;">
                    <span style="font-size:11px;text-transform:uppercase;letter-spacing:0.3em;color:#93c5fd;opacity:0.7;">Verify your email</span>
                  </td>
                </tr>
                <tr>
                  <td align="center" style="padding-bottom:16px;">
                    <h1 style="margin:0;font-size:26px;font-weight:600;letter-spacing:-0.02em;color:#ffffff;line-height:1.3;">
                      Welcome to Prism
                    </h1>
                  </td>
                </tr>
                <tr>
                  <td align="center" style="padding-bottom:20px;">
                    <p style="margin:0;font-size:15px;line-height:1.6;color:#a1a1aa;">
                      Hi <strong style="color:#e4e4e7;">{username}</strong>,<br>
                      Click the button below to verify your email address and start saving your best moments.
                    </p>
                  </td>
                </tr>

                <!-- Verification Button -->
                <tr>
                  <td align="center" style="padding-bottom:24px;">
                    <table role="presentation" cellpadding="0" cellspacing="0">
                      <tr>
                        <td style="border-radius:12px;background:linear-gradient(135deg,#6366f1,#4f46e5);padding:0;">
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
                      <a href="{verify_url}" style="color:#818cf8;text-decoration:underline;word-break:break-all;">{verify_url}</a>
                    </p>
                  </td>
                </tr>

                <tr>
                  <td style="border-top:1px solid #1f2a44;padding-top:20px;">
                    <p style="margin:0;font-size:13px;line-height:1.5;color:#52525b;">
                      This link expires in 24 hours. If you did not create an account on Prism, you can safely ignore this email.
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
        site_url = site_url,
        year = chrono::Utc::now().format("%Y"),
    )
}

pub async fn send_verification_email(
    config: &Config,
    to_email: &str,
    to_name: &str,
    token: &str,
) -> Result<(), String> {
    if config.smtp_host.is_empty() {
        tracing::error!(
            "SMTP not configured — verification email NOT sent to {to_email}. Token: {token}"
        );
        return Err("SMTP is not configured. Set SMTP_HOST, SMTP_USERNAME, and SMTP_PASSWORD.".into());
    }

    let verify_url = format!(
        "{}/verify-email?token={}",
        config.site_url.trim_end_matches('/'),
        token
    );

    let html = verification_email_html(to_name, &verify_url, &config.site_url);

    let from_name = &config.smtp_from_name;
    let from_address = &config.smtp_from_address;

    let email = Message::builder()
        .from(
            format!("{from_name} <{from_address}>")
                .parse()
                .map_err(|e| format!("Invalid from address: {e}"))?,
        )
        .to(format!("{to_name} <{to_email}>")
            .parse()
            .map_err(|e| format!("Invalid to address: {e}"))?)
        .subject("Verify your Prism account")
        .header(ContentType::TEXT_HTML)
        .body(html)
        .map_err(|e| format!("Failed to build email: {e}"))?;

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
        .map_err(|e| format!("SMTP relay error: {e}"))?
        .port(config.smtp_port)
        .credentials(creds)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| format!("Failed to send email: {e}"))?;

    tracing::info!(to = %to_email, "verification email sent");
    Ok(())
}
