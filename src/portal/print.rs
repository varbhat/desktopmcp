//! Print portal — print files using the desktop print dialog.

use anyhow::Result;
use ashpd::desktop::print::{PrintProxy, PreparePrintOptions, PrintOptions};

pub struct PrintPortal;

impl PrintPortal {
    /// Show the print dialog for a file (by path) and print it.
    ///
    /// Returns `true` if the user confirmed printing.
    pub async fn print_file(path: &str, title: &str) -> Result<bool> {
        use std::fs::File;

        let file = File::open(path)
            .map_err(|e| anyhow::anyhow!("Cannot open file '{}': {e}", path))?;

        let proxy = PrintProxy::new().await?;

        // Prepare: show the print dialog to get settings + token
        let prepare = proxy
            .prepare_print(
                None,
                title,
                Default::default(),
                Default::default(),
                PreparePrintOptions::default(),
            )
            .await?
            .response()?;

        // Print using the token from prepare
        let print_opts = PrintOptions::default().set_token(prepare.token);

        let result = proxy
            .print(None, title, &file, print_opts)
            .await?
            .response();

        Ok(result.is_ok())
    }
}
