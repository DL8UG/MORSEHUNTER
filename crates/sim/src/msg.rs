//! The messages stations exchange. A station never reads Morse: it is told what was sent.

/// Message kinds, in the order of the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Msg {
    None,
    Cq,
    Nr,
    Tu,
    MyCall,
    HisCall,
    B4,
    Qm,
    Nil,
    Garbage,
    RNr,
    RNr2,
    DeMyCall1,
    DeMyCall2,
    DeMyCallNr1,
    DeMyCallNr2,
    NrQm,
    LongCq,
    MyCallNr2,
    Qrl,
    Qrl2,
    Qsy,
    Agn,
}

impl Msg {
    /// `<my>`, `<his>` and `<#>` stand for the own call, the other call and the exchange.
    pub fn template(self) -> &'static str {
        match self {
            Msg::None | Msg::Garbage => "",
            Msg::Cq => "CQ <my> TEST",
            Msg::Nr => "<#>",
            Msg::Tu => "TU",
            Msg::MyCall => "<my>",
            Msg::HisCall => "<his>",
            Msg::B4 => "QSO B4",
            Msg::Qm => "?",
            Msg::Nil => "NIL",
            Msg::RNr => "R <#>",
            Msg::RNr2 => "R <#> <#>",
            Msg::DeMyCall1 => "DE <my>",
            Msg::DeMyCall2 => "DE <my> <my>",
            Msg::DeMyCallNr1 => "DE <my> <#>",
            Msg::DeMyCallNr2 => "DE <my> <my> <#>",
            Msg::NrQm => "NR?",
            Msg::LongCq => "CQ CQ TEST <my> <my> TEST",
            Msg::MyCallNr2 => "<my> <my> <#>",
            Msg::Qrl => "QRL?",
            Msg::Qrl2 => "QRL?   QRL?",
            Msg::Qsy => "<his>  QSY QSY",
            Msg::Agn => "AGN",
        }
    }
}

/// A set of messages: what went out in one transmission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MsgSet(u32);

impl MsgSet {
    pub const fn empty() -> Self {
        MsgSet(0)
    }

    pub fn of(m: Msg) -> Self {
        MsgSet(1 << m as u32)
    }

    pub fn add(&mut self, m: Msg) {
        self.0 |= 1 << m as u32;
    }

    pub fn has(self, m: Msg) -> bool {
        self.0 & 1 << m as u32 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}
