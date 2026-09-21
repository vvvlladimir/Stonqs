//! Supported ISO 10383 market identifiers and display names.
//!
//! A hand-picked subset of the ISO 10383 register — only the venues a registered quote source can
//! build a symbol for. The codes are the published standard's; the display names are shortened for
//! the interface and are not the register's official entries.

/// A supported market and its display name.
pub struct Market {
    pub mic: &'static str,
    pub name: &'static str,
}
/// Markets for which a registered quote source can construct symbols.
pub const MARKETS: &[Market] = &[
    Market {
        mic: "XETR",
        name: "Xetra",
    },
    Market {
        mic: "XFRA",
        name: "Frankfurt",
    },
    Market {
        mic: "XAMS",
        name: "Euronext Amsterdam",
    },
    Market {
        mic: "XPAR",
        name: "Euronext Paris",
    },
    Market {
        mic: "XMIL",
        name: "Borsa Italiana",
    },
    Market {
        mic: "XLON",
        name: "London Stock Exchange",
    },
    Market {
        mic: "XSWX",
        name: "SIX Swiss Exchange",
    },
    Market {
        mic: "XMAD",
        name: "Bolsa de Madrid",
    },
    Market {
        mic: "XBRU",
        name: "Euronext Brussels",
    },
    Market {
        mic: "XLIS",
        name: "Euronext Lisbon",
    },
    Market {
        mic: "XWBO",
        name: "Wiener Börse",
    },
    Market {
        mic: "XSTO",
        name: "Nasdaq Stockholm",
    },
    Market {
        mic: "XCSE",
        name: "Nasdaq Copenhagen",
    },
    Market {
        mic: "XHEL",
        name: "Nasdaq Helsinki",
    },
    Market {
        mic: "XOSL",
        name: "Oslo Børs",
    },
    Market {
        mic: "XWAR",
        name: "Warsaw Stock Exchange",
    },
    Market {
        mic: "XNAS",
        name: "Nasdaq",
    },
    Market {
        mic: "XNYS",
        name: "New York Stock Exchange",
    },
    Market {
        mic: "ARCX",
        name: "NYSE Arca",
    },
    Market {
        mic: "XTSE",
        name: "Toronto Stock Exchange",
    },
    Market {
        mic: "XHKG",
        name: "Hong Kong Stock Exchange",
    },
    Market {
        mic: "XTKS",
        name: "Tokyo Stock Exchange",
    },
    Market {
        mic: "XASX",
        name: "Australian Securities Exchange",
    },
    Market {
        mic: "XSES",
        name: "Singapore Exchange",
    },
    Market {
        mic: "XJSE",
        name: "Johannesburg Stock Exchange",
    },
    Market {
        mic: "XTAE",
        name: "Tel Aviv Stock Exchange",
    },
    Market {
        mic: "XMEX",
        name: "Bolsa Mexicana de Valores",
    },
    Market {
        mic: "BVMF",
        name: "B3 Brasil",
    },
    Market {
        mic: "XNSE",
        name: "National Stock Exchange of India",
    },
];
/// Resolve a supported MIC to its display name.
pub fn market_name(mic: &str) -> Option<&'static str> {
    MARKETS.iter().find(|m| m.mic == mic).map(|m| m.name)
}
/// Iterate over the supported MIC values.
pub fn supported_mics() -> impl Iterator<Item = &'static str> {
    MARKETS.iter().map(|m| m.mic)
}
