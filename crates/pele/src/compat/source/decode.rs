//! Pele src/text/entities.ts at 458322a5 (MIT, kepano). Per-call decoding, immutable name table.
use super::detect::{space, word};
use crate::compat::{ErrorCode, PeleError};
use std::{collections::BTreeMap, sync::OnceLock};
const NAMED:&str="amp:38,lt:60,gt:62,quot:34,apos:39,nbsp:160,iexcl:161,cent:162,pound:163,curren:164,yen:165,sect:167,copy:169,laquo:171,not:172,reg:174,deg:176,plusmn:177,sup2:178,sup3:179,micro:181,para:182,middot:183,sup1:185,raquo:187,frac14:188,frac12:189,frac34:190,iquest:191,times:215,divide:247,ndash:8211,mdash:8212,lsquo:8216,rsquo:8217,sbquo:8218,ldquo:8220,rdquo:8221,bdquo:8222,dagger:8224,Dagger:8225,bull:8226,hellip:8230,permil:8240,prime:8242,Prime:8243,lsaquo:8249,rsaquo:8250,euro:8364,trade:8482,larr:8592,uarr:8593,rarr:8594,darr:8595,harr:8596,lArr:8656,uArr:8657,rArr:8658,dArr:8659,hArr:8660,forall:8704,part:8706,exist:8707,empty:8709,nabla:8711,isin:8712,notin:8713,ni:8715,prod:8719,sum:8721,minus:8722,radic:8730,infin:8734,and:8743,or:8744,cap:8745,cup:8746,int:8747,there4:8756,sim:8764,cong:8773,asymp:8776,ne:8800,equiv:8801,le:8804,ge:8805,sub:8834,sup:8835,sube:8838,supe:8839,oplus:8853,otimes:8855,perp:8869,sdot:8901,loz:9674,spades:9824,clubs:9827,hearts:9829,diams:9830,check:10003,cross:10007,equals:61,colon:58,semi:59,num:35,percnt:37,lpar:40,rpar:41,lbrack:91,rbrack:93,lbrace:123,rbrace:125,vert:124,sol:47,bsol:92,quest:63,excl:33,ast:42,plus:43,comma:44,period:46,dollar:36,commat:64,grave:96,hat:94,lowbar:95,tilde:126,Alpha:913,Beta:914,Gamma:915,Delta:916,Epsilon:917,Zeta:918,Eta:919,Theta:920,Iota:921,Kappa:922,Lambda:923,Mu:924,Nu:925,Xi:926,Omicron:927,Pi:928,Rho:929,Sigma:931,Tau:932,Upsilon:933,Phi:934,Chi:935,Psi:936,Omega:937,alpha:945,beta:946,gamma:947,delta:948,epsilon:949,zeta:950,eta:951,theta:952,iota:953,kappa:954,lambda:955,mu:956,nu:957,xi:958,omicron:959,pi:960,rho:961,sigmaf:962,sigma:963,tau:964,upsilon:965,phi:966,chi:967,psi:968,omega:969";

fn lookup(name: &str) -> Option<char> {
    static NAMES: OnceLock<BTreeMap<&'static str, char>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            NAMED
                .split(',')
                .map(|pair| {
                    let (key, value) = pair.split_once(':').unwrap();
                    (key, char::from_u32(value.parse().unwrap()).unwrap())
                })
                .collect()
        })
        .get(name)
        .copied()
}
fn point(n: f64, fallback: &str) -> Result<String, PeleError> {
    if !(n > 0. && n <= 0x10ffff as f64) || (n >= 0xd800 as f64 && n <= 0xdfff as f64) {
        return Ok(fallback.into());
    }
    if n.fract() != 0. {
        return Err(PeleError::new(
            "Diagram is nested too deeply.",
            ErrorCode::Limit,
        ));
    }
    Ok(char::from_u32(n as u32).unwrap().to_string())
}
fn number(s: &str) -> f64 {
    let s = s.trim_matches(space);
    if s.is_empty() {
        return 0.;
    }
    for (prefix, base) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(s) = s.strip_prefix(prefix) {
            return if !s.is_empty() && s.chars().all(|c| c.is_digit(base)) {
                s.chars().fold(0., |v, c| {
                    v * base as f64 + c.to_digit(base).unwrap() as f64
                })
            } else {
                f64::NAN
            };
        }
    }
    s.parse().unwrap_or(f64::NAN)
}
pub(super) fn decode(source: &str) -> Result<String, PeleError> {
    let mut text = source.to_string();
    if text.contains("¶ß") {
        let mut out = String::new();
        let mut last = 0;
        let mut cursor = 0;
        while let Some(from) = text[cursor..].find("ﬂ°").map(|i| cursor + i) {
            let body_start = from + "ﬂ°".len();
            let Some(stop) = text[body_start..].find('¶').map(|i| body_start + i) else {
                break;
            };
            if text[stop..].starts_with("¶ß") {
                let numeric = text[body_start..].starts_with('°');
                let body = &text[body_start + if numeric { '°'.len_utf8() } else { 0 }..stop];
                let end = stop + "¶ß".len();
                out.push_str(&text[last..from]);
                if numeric {
                    out.push_str(&point(number(body), &text[from..end])?)
                } else if let Some(c) = lookup(body) {
                    out.push(c)
                } else {
                    out.push('&');
                    out.push_str(body);
                    out.push(';')
                }
                last = end;
            }
            cursor = stop + '¶'.len_utf8();
        }
        if last > 0 {
            out.push_str(&text[last..]);
            text = out
        }
    }
    if !text.contains('&') {
        return Ok(text);
    }
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(start) = text[cursor..].find('&').map(|i| cursor + i) {
        out.push_str(&text[cursor..start]);
        let from = start + 1;
        let (digits, base) = if text[from..].starts_with("#x") || text[from..].starts_with("#X") {
            (from + 2, 16)
        } else if text[from..].starts_with('#') {
            (from + 1, 10)
        } else {
            (from, 0)
        };
        let end = text[digits..]
            .char_indices()
            .find(|(_, c)| {
                if base == 0 {
                    !word(*c)
                } else {
                    !c.is_ascii() || !c.is_digit(base)
                }
            })
            .map_or(text.len(), |(i, _)| digits + i);
        if end > digits && text.as_bytes().get(end) == Some(&b';') {
            let body = &text[digits..end];
            let raw = &text[start..end + 1];
            if base == 0 {
                if let Some(c) = lookup(body) {
                    out.push(c)
                } else {
                    out.push_str(raw)
                }
            } else {
                let n = if base == 10 {
                    number(body)
                } else {
                    body.chars()
                        .fold(0., |v, c| v * 16. + c.to_digit(16).unwrap() as f64)
                };
                out.push_str(&point(n, raw)?)
            }
            cursor = end + 1;
        } else {
            out.push('&');
            cursor = from
        }
    }
    out.push_str(&text[cursor..]);
    Ok(out)
}
