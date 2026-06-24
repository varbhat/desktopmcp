use anyhow::Result;
use ashpd::desktop::file_chooser::{SelectedFiles, FileFilter};

pub struct FileChooserPortal;

impl FileChooserPortal {
    /// Open a file chooser dialog and return selected file URIs.
    pub async fn open_file(
        title: &str,
        multiple: bool,
        directory: bool,
        filters: Option<Vec<(&str, Vec<&str>)>>,
    ) -> Result<Vec<String>> {
        let mut req = SelectedFiles::open_file()
            .title(title)
            .modal(true)
            .multiple(multiple)
            .directory(directory);

        if let Some(filter_defs) = filters {
            for (label, patterns) in filter_defs {
                let mut filter = FileFilter::new(label);
                for pattern in patterns {
                    if pattern.contains('/') {
                        filter = filter.mimetype(pattern);
                    } else {
                        filter = filter.glob(pattern);
                    }
                }
                req = req.filter(filter);
            }
        }

        let response = req.send().await?.response()?;
        let uris: Vec<String> = response.uris().iter().map(|u| u.to_string()).collect();
        tracing::info!("File chooser returned {} file(s)", uris.len());
        Ok(uris)
    }

    /// Open a save-file dialog and return the chosen path.
    pub async fn save_file(
        title: &str,
        current_name: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut req = SelectedFiles::save_file()
            .title(title)
            .modal(true);

        if let Some(name) = current_name {
            req = req.current_name(name);
        }

        let response = req.send().await?.response()?;
        let uris: Vec<String> = response.uris().iter().map(|u| u.to_string()).collect();
        tracing::info!("Save file dialog returned: {:?}", uris);
        Ok(uris)
    }

    /// Open a save-multiple-files dialog.
    ///
    /// `filenames` is a list of suggested file names (no paths).
    pub async fn save_files(
        title: &str,
        filenames: Vec<String>,
    ) -> Result<Vec<String>> {
        let req = SelectedFiles::save_files()
            .title(title)
            .files(filenames.iter().map(|s| s.as_str()))?;

        let response = req.send().await?.response()?;
        let uris: Vec<String> = response.uris().iter().map(|u| u.to_string()).collect();
        Ok(uris)
    }
}
