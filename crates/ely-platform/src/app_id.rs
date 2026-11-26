/// The eight apps, in PRD order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppId {
    Doc,
    Table,
    Show,
    Pdf,
    Mail,
    Tree,
    Mark,
    Agent,
}

impl AppId {
    pub const ALL: [AppId; 8] = [
        AppId::Doc,
        AppId::Table,
        AppId::Show,
        AppId::Pdf,
        AppId::Mail,
        AppId::Tree,
        AppId::Mark,
        AppId::Agent,
    ];
    /// The apps on the shared kernel, with idle engines.
    pub const DOCUMENTS: [AppId; 3] = [AppId::Doc, AppId::Table, AppId::Show];

    /// The macOS bundle identifier `packaging/macos/bundle.sh` writes.
    pub fn bundle_id(self) -> String {
        match self {
            // Green is its own product, not an Ely Office app.
            AppId::Mark => "com.fenghu.green".into(),
            other => format!("ely.zacharyzhang.office.{}", &other.slug()["ely-".len()..]),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AppId::Doc => "ElyDoc",
            AppId::Table => "ElyTable",
            AppId::Show => "ElyShow",
            AppId::Pdf => "ElyPDF",
            AppId::Mail => "ElyMail",
            AppId::Tree => "ElyTree",
            AppId::Mark => "Green",
            AppId::Agent => "ElyAgent",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            AppId::Doc => "ely-doc",
            AppId::Table => "ely-table",
            AppId::Show => "ely-show",
            AppId::Pdf => "ely-pdf",
            AppId::Mail => "ely-mail",
            AppId::Tree => "ely-tree",
            AppId::Mark => "green",
            AppId::Agent => "ely-agent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppId::{self, *};

    const IN_PRD_ORDER: [AppId; 8] = AppId::ALL;

    #[test]
    fn bundle_ids_match_the_packaging() {
        assert_eq!(Doc.bundle_id(), "ely.zacharyzhang.office.doc");
        assert_eq!(Mark.bundle_id(), "com.fenghu.green");
        let script = include_str!("../../../packaging/macos/bundle.sh");
        assert!(script.contains("prefix=ely.zacharyzhang.office\n"));
        assert!(script.contains("<string>$prefix.${slug#ely-}</string>"));
    }

    #[test]
    fn names_follow_the_prd() {
        let names: Vec<_> = IN_PRD_ORDER.iter().map(|app| app.name()).collect();
        assert_eq!(
            names,
            [
                "ElyDoc", "ElyTable", "ElyShow", "ElyPDF", "ElyMail", "ElyTree", "Green",
                "ElyAgent"
            ]
        );
    }

    #[test]
    fn slugs_are_the_binary_names() {
        let slugs: Vec<_> = IN_PRD_ORDER.iter().map(|app| app.slug()).collect();
        assert_eq!(
            slugs,
            [
                "ely-doc",
                "ely-table",
                "ely-show",
                "ely-pdf",
                "ely-mail",
                "ely-tree",
                "green",
                "ely-agent"
            ]
        );
    }
}
