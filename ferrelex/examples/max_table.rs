// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

extern crate ferrelex;

use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer,
};

/// This example tests that `DecisionTree::Table` entries never exceed `u8::MAX`.
///
/// The lexer has 256 individual single-character patterns for U+0100–U+01FF.
/// The DFA initial state therefore has 256 distinct partitions:
///
/// - Partitions 0–254 go into the table (values 1–255, i.e. up to `u8::MAX`).
/// - Partition 255 is excluded by the `i >= 255` guard and handled by the
///   decision tree instead.
///
/// The critical case is U+01FE (partition 254, table value 255). If the table
/// storage used a wrapping cast instead of `strict_add`, that entry would
/// silently become 0 and U+01FE would be misclassified as `Other`.
///
/// Removing the `i >= 255` guard entirely would panic at macro-expansion time
/// because `(255u8).strict_add(1)` overflows.
#[derive(Debug, PartialEq, Clone)]
enum Token {
    Latin,
    Other,
}

lexer::lex! {
    const C000: Regex = '\u{0100}'; const C001: Regex = '\u{0101}';
    const C002: Regex = '\u{0102}'; const C003: Regex = '\u{0103}';
    const C004: Regex = '\u{0104}'; const C005: Regex = '\u{0105}';
    const C006: Regex = '\u{0106}'; const C007: Regex = '\u{0107}';
    const C008: Regex = '\u{0108}'; const C009: Regex = '\u{0109}';
    const C010: Regex = '\u{010A}'; const C011: Regex = '\u{010B}';
    const C012: Regex = '\u{010C}'; const C013: Regex = '\u{010D}';
    const C014: Regex = '\u{010E}'; const C015: Regex = '\u{010F}';
    const C016: Regex = '\u{0110}'; const C017: Regex = '\u{0111}';
    const C018: Regex = '\u{0112}'; const C019: Regex = '\u{0113}';
    const C020: Regex = '\u{0114}'; const C021: Regex = '\u{0115}';
    const C022: Regex = '\u{0116}'; const C023: Regex = '\u{0117}';
    const C024: Regex = '\u{0118}'; const C025: Regex = '\u{0119}';
    const C026: Regex = '\u{011A}'; const C027: Regex = '\u{011B}';
    const C028: Regex = '\u{011C}'; const C029: Regex = '\u{011D}';
    const C030: Regex = '\u{011E}'; const C031: Regex = '\u{011F}';
    const C032: Regex = '\u{0120}'; const C033: Regex = '\u{0121}';
    const C034: Regex = '\u{0122}'; const C035: Regex = '\u{0123}';
    const C036: Regex = '\u{0124}'; const C037: Regex = '\u{0125}';
    const C038: Regex = '\u{0126}'; const C039: Regex = '\u{0127}';
    const C040: Regex = '\u{0128}'; const C041: Regex = '\u{0129}';
    const C042: Regex = '\u{012A}'; const C043: Regex = '\u{012B}';
    const C044: Regex = '\u{012C}'; const C045: Regex = '\u{012D}';
    const C046: Regex = '\u{012E}'; const C047: Regex = '\u{012F}';
    const C048: Regex = '\u{0130}'; const C049: Regex = '\u{0131}';
    const C050: Regex = '\u{0132}'; const C051: Regex = '\u{0133}';
    const C052: Regex = '\u{0134}'; const C053: Regex = '\u{0135}';
    const C054: Regex = '\u{0136}'; const C055: Regex = '\u{0137}';
    const C056: Regex = '\u{0138}'; const C057: Regex = '\u{0139}';
    const C058: Regex = '\u{013A}'; const C059: Regex = '\u{013B}';
    const C060: Regex = '\u{013C}'; const C061: Regex = '\u{013D}';
    const C062: Regex = '\u{013E}'; const C063: Regex = '\u{013F}';
    const C064: Regex = '\u{0140}'; const C065: Regex = '\u{0141}';
    const C066: Regex = '\u{0142}'; const C067: Regex = '\u{0143}';
    const C068: Regex = '\u{0144}'; const C069: Regex = '\u{0145}';
    const C070: Regex = '\u{0146}'; const C071: Regex = '\u{0147}';
    const C072: Regex = '\u{0148}'; const C073: Regex = '\u{0149}';
    const C074: Regex = '\u{014A}'; const C075: Regex = '\u{014B}';
    const C076: Regex = '\u{014C}'; const C077: Regex = '\u{014D}';
    const C078: Regex = '\u{014E}'; const C079: Regex = '\u{014F}';
    const C080: Regex = '\u{0150}'; const C081: Regex = '\u{0151}';
    const C082: Regex = '\u{0152}'; const C083: Regex = '\u{0153}';
    const C084: Regex = '\u{0154}'; const C085: Regex = '\u{0155}';
    const C086: Regex = '\u{0156}'; const C087: Regex = '\u{0157}';
    const C088: Regex = '\u{0158}'; const C089: Regex = '\u{0159}';
    const C090: Regex = '\u{015A}'; const C091: Regex = '\u{015B}';
    const C092: Regex = '\u{015C}'; const C093: Regex = '\u{015D}';
    const C094: Regex = '\u{015E}'; const C095: Regex = '\u{015F}';
    const C096: Regex = '\u{0160}'; const C097: Regex = '\u{0161}';
    const C098: Regex = '\u{0162}'; const C099: Regex = '\u{0163}';
    const C100: Regex = '\u{0164}'; const C101: Regex = '\u{0165}';
    const C102: Regex = '\u{0166}'; const C103: Regex = '\u{0167}';
    const C104: Regex = '\u{0168}'; const C105: Regex = '\u{0169}';
    const C106: Regex = '\u{016A}'; const C107: Regex = '\u{016B}';
    const C108: Regex = '\u{016C}'; const C109: Regex = '\u{016D}';
    const C110: Regex = '\u{016E}'; const C111: Regex = '\u{016F}';
    const C112: Regex = '\u{0170}'; const C113: Regex = '\u{0171}';
    const C114: Regex = '\u{0172}'; const C115: Regex = '\u{0173}';
    const C116: Regex = '\u{0174}'; const C117: Regex = '\u{0175}';
    const C118: Regex = '\u{0176}'; const C119: Regex = '\u{0177}';
    const C120: Regex = '\u{0178}'; const C121: Regex = '\u{0179}';
    const C122: Regex = '\u{017A}'; const C123: Regex = '\u{017B}';
    const C124: Regex = '\u{017C}'; const C125: Regex = '\u{017D}';
    const C126: Regex = '\u{017E}'; const C127: Regex = '\u{017F}';
    const C128: Regex = '\u{0180}'; const C129: Regex = '\u{0181}';
    const C130: Regex = '\u{0182}'; const C131: Regex = '\u{0183}';
    const C132: Regex = '\u{0184}'; const C133: Regex = '\u{0185}';
    const C134: Regex = '\u{0186}'; const C135: Regex = '\u{0187}';
    const C136: Regex = '\u{0188}'; const C137: Regex = '\u{0189}';
    const C138: Regex = '\u{018A}'; const C139: Regex = '\u{018B}';
    const C140: Regex = '\u{018C}'; const C141: Regex = '\u{018D}';
    const C142: Regex = '\u{018E}'; const C143: Regex = '\u{018F}';
    const C144: Regex = '\u{0190}'; const C145: Regex = '\u{0191}';
    const C146: Regex = '\u{0192}'; const C147: Regex = '\u{0193}';
    const C148: Regex = '\u{0194}'; const C149: Regex = '\u{0195}';
    const C150: Regex = '\u{0196}'; const C151: Regex = '\u{0197}';
    const C152: Regex = '\u{0198}'; const C153: Regex = '\u{0199}';
    const C154: Regex = '\u{019A}'; const C155: Regex = '\u{019B}';
    const C156: Regex = '\u{019C}'; const C157: Regex = '\u{019D}';
    const C158: Regex = '\u{019E}'; const C159: Regex = '\u{019F}';
    const C160: Regex = '\u{01A0}'; const C161: Regex = '\u{01A1}';
    const C162: Regex = '\u{01A2}'; const C163: Regex = '\u{01A3}';
    const C164: Regex = '\u{01A4}'; const C165: Regex = '\u{01A5}';
    const C166: Regex = '\u{01A6}'; const C167: Regex = '\u{01A7}';
    const C168: Regex = '\u{01A8}'; const C169: Regex = '\u{01A9}';
    const C170: Regex = '\u{01AA}'; const C171: Regex = '\u{01AB}';
    const C172: Regex = '\u{01AC}'; const C173: Regex = '\u{01AD}';
    const C174: Regex = '\u{01AE}'; const C175: Regex = '\u{01AF}';
    const C176: Regex = '\u{01B0}'; const C177: Regex = '\u{01B1}';
    const C178: Regex = '\u{01B2}'; const C179: Regex = '\u{01B3}';
    const C180: Regex = '\u{01B4}'; const C181: Regex = '\u{01B5}';
    const C182: Regex = '\u{01B6}'; const C183: Regex = '\u{01B7}';
    const C184: Regex = '\u{01B8}'; const C185: Regex = '\u{01B9}';
    const C186: Regex = '\u{01BA}'; const C187: Regex = '\u{01BB}';
    const C188: Regex = '\u{01BC}'; const C189: Regex = '\u{01BD}';
    const C190: Regex = '\u{01BE}'; const C191: Regex = '\u{01BF}';
    const C192: Regex = '\u{01C0}'; const C193: Regex = '\u{01C1}';
    const C194: Regex = '\u{01C2}'; const C195: Regex = '\u{01C3}';
    const C196: Regex = '\u{01C4}'; const C197: Regex = '\u{01C5}';
    const C198: Regex = '\u{01C6}'; const C199: Regex = '\u{01C7}';
    const C200: Regex = '\u{01C8}'; const C201: Regex = '\u{01C9}';
    const C202: Regex = '\u{01CA}'; const C203: Regex = '\u{01CB}';
    const C204: Regex = '\u{01CC}'; const C205: Regex = '\u{01CD}';
    const C206: Regex = '\u{01CE}'; const C207: Regex = '\u{01CF}';
    const C208: Regex = '\u{01D0}'; const C209: Regex = '\u{01D1}';
    const C210: Regex = '\u{01D2}'; const C211: Regex = '\u{01D3}';
    const C212: Regex = '\u{01D4}'; const C213: Regex = '\u{01D5}';
    const C214: Regex = '\u{01D6}'; const C215: Regex = '\u{01D7}';
    const C216: Regex = '\u{01D8}'; const C217: Regex = '\u{01D9}';
    const C218: Regex = '\u{01DA}'; const C219: Regex = '\u{01DB}';
    const C220: Regex = '\u{01DC}'; const C221: Regex = '\u{01DD}';
    const C222: Regex = '\u{01DE}'; const C223: Regex = '\u{01DF}';
    const C224: Regex = '\u{01E0}'; const C225: Regex = '\u{01E1}';
    const C226: Regex = '\u{01E2}'; const C227: Regex = '\u{01E3}';
    const C228: Regex = '\u{01E4}'; const C229: Regex = '\u{01E5}';
    const C230: Regex = '\u{01E6}'; const C231: Regex = '\u{01E7}';
    const C232: Regex = '\u{01E8}'; const C233: Regex = '\u{01E9}';
    const C234: Regex = '\u{01EA}'; const C235: Regex = '\u{01EB}';
    const C236: Regex = '\u{01EC}'; const C237: Regex = '\u{01ED}';
    const C238: Regex = '\u{01EE}'; const C239: Regex = '\u{01EF}';
    const C240: Regex = '\u{01F0}'; const C241: Regex = '\u{01F1}';
    const C242: Regex = '\u{01F2}'; const C243: Regex = '\u{01F3}';
    const C244: Regex = '\u{01F4}'; const C245: Regex = '\u{01F5}';
    const C246: Regex = '\u{01F6}'; const C247: Regex = '\u{01F7}';
    const C248: Regex = '\u{01F8}'; const C249: Regex = '\u{01F9}';
    const C250: Regex = '\u{01FA}'; const C251: Regex = '\u{01FB}';
    const C252: Regex = '\u{01FC}'; const C253: Regex = '\u{01FD}';
    // C254 (partition 254) -> table value 255 = u8::MAX
    const C254: Regex = '\u{01FE}';
    // C255 (partition 255) -> excluded from table by i >= 255 guard, handled by decision tree
    const C255: Regex = '\u{01FF}';
    // C256-C511: partitions 256-511 (U+0200-U+02FF).
    const C256: Regex = '\u{0200}'; const C257: Regex = '\u{0201}';
    const C258: Regex = '\u{0202}'; const C259: Regex = '\u{0203}';
    const C260: Regex = '\u{0204}'; const C261: Regex = '\u{0205}';
    const C262: Regex = '\u{0206}'; const C263: Regex = '\u{0207}';
    const C264: Regex = '\u{0208}'; const C265: Regex = '\u{0209}';
    const C266: Regex = '\u{020A}'; const C267: Regex = '\u{020B}';
    const C268: Regex = '\u{020C}'; const C269: Regex = '\u{020D}';
    const C270: Regex = '\u{020E}'; const C271: Regex = '\u{020F}';
    const C272: Regex = '\u{0210}'; const C273: Regex = '\u{0211}';
    const C274: Regex = '\u{0212}'; const C275: Regex = '\u{0213}';
    const C276: Regex = '\u{0214}'; const C277: Regex = '\u{0215}';
    const C278: Regex = '\u{0216}'; const C279: Regex = '\u{0217}';
    const C280: Regex = '\u{0218}'; const C281: Regex = '\u{0219}';
    const C282: Regex = '\u{021A}'; const C283: Regex = '\u{021B}';
    const C284: Regex = '\u{021C}'; const C285: Regex = '\u{021D}';
    const C286: Regex = '\u{021E}'; const C287: Regex = '\u{021F}';
    const C288: Regex = '\u{0220}'; const C289: Regex = '\u{0221}';
    const C290: Regex = '\u{0222}'; const C291: Regex = '\u{0223}';
    const C292: Regex = '\u{0224}'; const C293: Regex = '\u{0225}';
    const C294: Regex = '\u{0226}'; const C295: Regex = '\u{0227}';
    const C296: Regex = '\u{0228}'; const C297: Regex = '\u{0229}';
    const C298: Regex = '\u{022A}'; const C299: Regex = '\u{022B}';
    const C300: Regex = '\u{022C}'; const C301: Regex = '\u{022D}';
    const C302: Regex = '\u{022E}'; const C303: Regex = '\u{022F}';
    const C304: Regex = '\u{0230}'; const C305: Regex = '\u{0231}';
    const C306: Regex = '\u{0232}'; const C307: Regex = '\u{0233}';
    const C308: Regex = '\u{0234}'; const C309: Regex = '\u{0235}';
    const C310: Regex = '\u{0236}'; const C311: Regex = '\u{0237}';
    const C312: Regex = '\u{0238}'; const C313: Regex = '\u{0239}';
    const C314: Regex = '\u{023A}'; const C315: Regex = '\u{023B}';
    const C316: Regex = '\u{023C}'; const C317: Regex = '\u{023D}';
    const C318: Regex = '\u{023E}'; const C319: Regex = '\u{023F}';
    const C320: Regex = '\u{0240}'; const C321: Regex = '\u{0241}';
    const C322: Regex = '\u{0242}'; const C323: Regex = '\u{0243}';
    const C324: Regex = '\u{0244}'; const C325: Regex = '\u{0245}';
    const C326: Regex = '\u{0246}'; const C327: Regex = '\u{0247}';
    const C328: Regex = '\u{0248}'; const C329: Regex = '\u{0249}';
    const C330: Regex = '\u{024A}'; const C331: Regex = '\u{024B}';
    const C332: Regex = '\u{024C}'; const C333: Regex = '\u{024D}';
    const C334: Regex = '\u{024E}'; const C335: Regex = '\u{024F}';
    const C336: Regex = '\u{0250}'; const C337: Regex = '\u{0251}';
    const C338: Regex = '\u{0252}'; const C339: Regex = '\u{0253}';
    const C340: Regex = '\u{0254}'; const C341: Regex = '\u{0255}';
    const C342: Regex = '\u{0256}'; const C343: Regex = '\u{0257}';
    const C344: Regex = '\u{0258}'; const C345: Regex = '\u{0259}';
    const C346: Regex = '\u{025A}'; const C347: Regex = '\u{025B}';
    const C348: Regex = '\u{025C}'; const C349: Regex = '\u{025D}';
    const C350: Regex = '\u{025E}'; const C351: Regex = '\u{025F}';
    const C352: Regex = '\u{0260}'; const C353: Regex = '\u{0261}';
    const C354: Regex = '\u{0262}'; const C355: Regex = '\u{0263}';
    const C356: Regex = '\u{0264}'; const C357: Regex = '\u{0265}';
    const C358: Regex = '\u{0266}'; const C359: Regex = '\u{0267}';
    const C360: Regex = '\u{0268}'; const C361: Regex = '\u{0269}';
    const C362: Regex = '\u{026A}'; const C363: Regex = '\u{026B}';
    const C364: Regex = '\u{026C}'; const C365: Regex = '\u{026D}';
    const C366: Regex = '\u{026E}'; const C367: Regex = '\u{026F}';
    const C368: Regex = '\u{0270}'; const C369: Regex = '\u{0271}';
    const C370: Regex = '\u{0272}'; const C371: Regex = '\u{0273}';
    const C372: Regex = '\u{0274}'; const C373: Regex = '\u{0275}';
    const C374: Regex = '\u{0276}'; const C375: Regex = '\u{0277}';
    const C376: Regex = '\u{0278}'; const C377: Regex = '\u{0279}';
    const C378: Regex = '\u{027A}'; const C379: Regex = '\u{027B}';
    const C380: Regex = '\u{027C}'; const C381: Regex = '\u{027D}';
    const C382: Regex = '\u{027E}'; const C383: Regex = '\u{027F}';
    const C384: Regex = '\u{0280}'; const C385: Regex = '\u{0281}';
    const C386: Regex = '\u{0282}'; const C387: Regex = '\u{0283}';
    const C388: Regex = '\u{0284}'; const C389: Regex = '\u{0285}';
    const C390: Regex = '\u{0286}'; const C391: Regex = '\u{0287}';
    const C392: Regex = '\u{0288}'; const C393: Regex = '\u{0289}';
    const C394: Regex = '\u{028A}'; const C395: Regex = '\u{028B}';
    const C396: Regex = '\u{028C}'; const C397: Regex = '\u{028D}';
    const C398: Regex = '\u{028E}'; const C399: Regex = '\u{028F}';
    const C400: Regex = '\u{0290}'; const C401: Regex = '\u{0291}';
    const C402: Regex = '\u{0292}'; const C403: Regex = '\u{0293}';
    const C404: Regex = '\u{0294}'; const C405: Regex = '\u{0295}';
    const C406: Regex = '\u{0296}'; const C407: Regex = '\u{0297}';
    const C408: Regex = '\u{0298}'; const C409: Regex = '\u{0299}';
    const C410: Regex = '\u{029A}'; const C411: Regex = '\u{029B}';
    const C412: Regex = '\u{029C}'; const C413: Regex = '\u{029D}';
    const C414: Regex = '\u{029E}'; const C415: Regex = '\u{029F}';
    const C416: Regex = '\u{02A0}'; const C417: Regex = '\u{02A1}';
    const C418: Regex = '\u{02A2}'; const C419: Regex = '\u{02A3}';
    const C420: Regex = '\u{02A4}'; const C421: Regex = '\u{02A5}';
    const C422: Regex = '\u{02A6}'; const C423: Regex = '\u{02A7}';
    const C424: Regex = '\u{02A8}'; const C425: Regex = '\u{02A9}';
    const C426: Regex = '\u{02AA}'; const C427: Regex = '\u{02AB}';
    const C428: Regex = '\u{02AC}'; const C429: Regex = '\u{02AD}';
    const C430: Regex = '\u{02AE}'; const C431: Regex = '\u{02AF}';
    const C432: Regex = '\u{02B0}'; const C433: Regex = '\u{02B1}';
    const C434: Regex = '\u{02B2}'; const C435: Regex = '\u{02B3}';
    const C436: Regex = '\u{02B4}'; const C437: Regex = '\u{02B5}';
    const C438: Regex = '\u{02B6}'; const C439: Regex = '\u{02B7}';
    const C440: Regex = '\u{02B8}'; const C441: Regex = '\u{02B9}';
    const C442: Regex = '\u{02BA}'; const C443: Regex = '\u{02BB}';
    const C444: Regex = '\u{02BC}'; const C445: Regex = '\u{02BD}';
    const C446: Regex = '\u{02BE}'; const C447: Regex = '\u{02BF}';
    const C448: Regex = '\u{02C0}'; const C449: Regex = '\u{02C1}';
    const C450: Regex = '\u{02C2}'; const C451: Regex = '\u{02C3}';
    const C452: Regex = '\u{02C4}'; const C453: Regex = '\u{02C5}';
    const C454: Regex = '\u{02C6}'; const C455: Regex = '\u{02C7}';
    const C456: Regex = '\u{02C8}'; const C457: Regex = '\u{02C9}';
    const C458: Regex = '\u{02CA}'; const C459: Regex = '\u{02CB}';
    const C460: Regex = '\u{02CC}'; const C461: Regex = '\u{02CD}';
    const C462: Regex = '\u{02CE}'; const C463: Regex = '\u{02CF}';
    const C464: Regex = '\u{02D0}'; const C465: Regex = '\u{02D1}';
    const C466: Regex = '\u{02D2}'; const C467: Regex = '\u{02D3}';
    const C468: Regex = '\u{02D4}'; const C469: Regex = '\u{02D5}';
    const C470: Regex = '\u{02D6}'; const C471: Regex = '\u{02D7}';
    const C472: Regex = '\u{02D8}'; const C473: Regex = '\u{02D9}';
    const C474: Regex = '\u{02DA}'; const C475: Regex = '\u{02DB}';
    const C476: Regex = '\u{02DC}'; const C477: Regex = '\u{02DD}';
    const C478: Regex = '\u{02DE}'; const C479: Regex = '\u{02DF}';
    const C480: Regex = '\u{02E0}'; const C481: Regex = '\u{02E1}';
    const C482: Regex = '\u{02E2}'; const C483: Regex = '\u{02E3}';
    const C484: Regex = '\u{02E4}'; const C485: Regex = '\u{02E5}';
    const C486: Regex = '\u{02E6}'; const C487: Regex = '\u{02E7}';
    const C488: Regex = '\u{02E8}'; const C489: Regex = '\u{02E9}';
    const C490: Regex = '\u{02EA}'; const C491: Regex = '\u{02EB}';
    const C492: Regex = '\u{02EC}'; const C493: Regex = '\u{02ED}';
    const C494: Regex = '\u{02EE}'; const C495: Regex = '\u{02EF}';
    const C496: Regex = '\u{02F0}'; const C497: Regex = '\u{02F1}';
    const C498: Regex = '\u{02F2}'; const C499: Regex = '\u{02F3}';
    const C500: Regex = '\u{02F4}'; const C501: Regex = '\u{02F5}';
    const C502: Regex = '\u{02F6}'; const C503: Regex = '\u{02F7}';
    const C504: Regex = '\u{02F8}'; const C505: Regex = '\u{02F9}';
    const C506: Regex = '\u{02FA}'; const C507: Regex = '\u{02FB}';
    const C508: Regex = '\u{02FC}'; const C509: Regex = '\u{02FD}';
    const C510: Regex = '\u{02FE}'; const C511: Regex = '\u{02FF}';

    pub fn lex(lexbuf: &mut LexBuf) -> Token {
        #[lexer]
        match lexbuf {
            C000 => Token::Latin, C001 => Token::Latin, C002 => Token::Latin, C003 => Token::Latin,
            C004 => Token::Latin, C005 => Token::Latin, C006 => Token::Latin, C007 => Token::Latin,
            C008 => Token::Latin, C009 => Token::Latin, C010 => Token::Latin, C011 => Token::Latin,
            C012 => Token::Latin, C013 => Token::Latin, C014 => Token::Latin, C015 => Token::Latin,
            C016 => Token::Latin, C017 => Token::Latin, C018 => Token::Latin, C019 => Token::Latin,
            C020 => Token::Latin, C021 => Token::Latin, C022 => Token::Latin, C023 => Token::Latin,
            C024 => Token::Latin, C025 => Token::Latin, C026 => Token::Latin, C027 => Token::Latin,
            C028 => Token::Latin, C029 => Token::Latin, C030 => Token::Latin, C031 => Token::Latin,
            C032 => Token::Latin, C033 => Token::Latin, C034 => Token::Latin, C035 => Token::Latin,
            C036 => Token::Latin, C037 => Token::Latin, C038 => Token::Latin, C039 => Token::Latin,
            C040 => Token::Latin, C041 => Token::Latin, C042 => Token::Latin, C043 => Token::Latin,
            C044 => Token::Latin, C045 => Token::Latin, C046 => Token::Latin, C047 => Token::Latin,
            C048 => Token::Latin, C049 => Token::Latin, C050 => Token::Latin, C051 => Token::Latin,
            C052 => Token::Latin, C053 => Token::Latin, C054 => Token::Latin, C055 => Token::Latin,
            C056 => Token::Latin, C057 => Token::Latin, C058 => Token::Latin, C059 => Token::Latin,
            C060 => Token::Latin, C061 => Token::Latin, C062 => Token::Latin, C063 => Token::Latin,
            C064 => Token::Latin, C065 => Token::Latin, C066 => Token::Latin, C067 => Token::Latin,
            C068 => Token::Latin, C069 => Token::Latin, C070 => Token::Latin, C071 => Token::Latin,
            C072 => Token::Latin, C073 => Token::Latin, C074 => Token::Latin, C075 => Token::Latin,
            C076 => Token::Latin, C077 => Token::Latin, C078 => Token::Latin, C079 => Token::Latin,
            C080 => Token::Latin, C081 => Token::Latin, C082 => Token::Latin, C083 => Token::Latin,
            C084 => Token::Latin, C085 => Token::Latin, C086 => Token::Latin, C087 => Token::Latin,
            C088 => Token::Latin, C089 => Token::Latin, C090 => Token::Latin, C091 => Token::Latin,
            C092 => Token::Latin, C093 => Token::Latin, C094 => Token::Latin, C095 => Token::Latin,
            C096 => Token::Latin, C097 => Token::Latin, C098 => Token::Latin, C099 => Token::Latin,
            C100 => Token::Latin, C101 => Token::Latin, C102 => Token::Latin, C103 => Token::Latin,
            C104 => Token::Latin, C105 => Token::Latin, C106 => Token::Latin, C107 => Token::Latin,
            C108 => Token::Latin, C109 => Token::Latin, C110 => Token::Latin, C111 => Token::Latin,
            C112 => Token::Latin, C113 => Token::Latin, C114 => Token::Latin, C115 => Token::Latin,
            C116 => Token::Latin, C117 => Token::Latin, C118 => Token::Latin, C119 => Token::Latin,
            C120 => Token::Latin, C121 => Token::Latin, C122 => Token::Latin, C123 => Token::Latin,
            C124 => Token::Latin, C125 => Token::Latin, C126 => Token::Latin, C127 => Token::Latin,
            C128 => Token::Latin, C129 => Token::Latin, C130 => Token::Latin, C131 => Token::Latin,
            C132 => Token::Latin, C133 => Token::Latin, C134 => Token::Latin, C135 => Token::Latin,
            C136 => Token::Latin, C137 => Token::Latin, C138 => Token::Latin, C139 => Token::Latin,
            C140 => Token::Latin, C141 => Token::Latin, C142 => Token::Latin, C143 => Token::Latin,
            C144 => Token::Latin, C145 => Token::Latin, C146 => Token::Latin, C147 => Token::Latin,
            C148 => Token::Latin, C149 => Token::Latin, C150 => Token::Latin, C151 => Token::Latin,
            C152 => Token::Latin, C153 => Token::Latin, C154 => Token::Latin, C155 => Token::Latin,
            C156 => Token::Latin, C157 => Token::Latin, C158 => Token::Latin, C159 => Token::Latin,
            C160 => Token::Latin, C161 => Token::Latin, C162 => Token::Latin, C163 => Token::Latin,
            C164 => Token::Latin, C165 => Token::Latin, C166 => Token::Latin, C167 => Token::Latin,
            C168 => Token::Latin, C169 => Token::Latin, C170 => Token::Latin, C171 => Token::Latin,
            C172 => Token::Latin, C173 => Token::Latin, C174 => Token::Latin, C175 => Token::Latin,
            C176 => Token::Latin, C177 => Token::Latin, C178 => Token::Latin, C179 => Token::Latin,
            C180 => Token::Latin, C181 => Token::Latin, C182 => Token::Latin, C183 => Token::Latin,
            C184 => Token::Latin, C185 => Token::Latin, C186 => Token::Latin, C187 => Token::Latin,
            C188 => Token::Latin, C189 => Token::Latin, C190 => Token::Latin, C191 => Token::Latin,
            C192 => Token::Latin, C193 => Token::Latin, C194 => Token::Latin, C195 => Token::Latin,
            C196 => Token::Latin, C197 => Token::Latin, C198 => Token::Latin, C199 => Token::Latin,
            C200 => Token::Latin, C201 => Token::Latin, C202 => Token::Latin, C203 => Token::Latin,
            C204 => Token::Latin, C205 => Token::Latin, C206 => Token::Latin, C207 => Token::Latin,
            C208 => Token::Latin, C209 => Token::Latin, C210 => Token::Latin, C211 => Token::Latin,
            C212 => Token::Latin, C213 => Token::Latin, C214 => Token::Latin, C215 => Token::Latin,
            C216 => Token::Latin, C217 => Token::Latin, C218 => Token::Latin, C219 => Token::Latin,
            C220 => Token::Latin, C221 => Token::Latin, C222 => Token::Latin, C223 => Token::Latin,
            C224 => Token::Latin, C225 => Token::Latin, C226 => Token::Latin, C227 => Token::Latin,
            C228 => Token::Latin, C229 => Token::Latin, C230 => Token::Latin, C231 => Token::Latin,
            C232 => Token::Latin, C233 => Token::Latin, C234 => Token::Latin, C235 => Token::Latin,
            C236 => Token::Latin, C237 => Token::Latin, C238 => Token::Latin, C239 => Token::Latin,
            C240 => Token::Latin, C241 => Token::Latin, C242 => Token::Latin, C243 => Token::Latin,
            C244 => Token::Latin, C245 => Token::Latin, C246 => Token::Latin, C247 => Token::Latin,
            C248 => Token::Latin, C249 => Token::Latin, C250 => Token::Latin, C251 => Token::Latin,
            C252 => Token::Latin, C253 => Token::Latin, C254 => Token::Latin, C255 => Token::Latin,
            C256 => Token::Latin, C257 => Token::Latin, C258 => Token::Latin, C259 => Token::Latin,
            C260 => Token::Latin, C261 => Token::Latin, C262 => Token::Latin, C263 => Token::Latin,
            C264 => Token::Latin, C265 => Token::Latin, C266 => Token::Latin, C267 => Token::Latin,
            C268 => Token::Latin, C269 => Token::Latin, C270 => Token::Latin, C271 => Token::Latin,
            C272 => Token::Latin, C273 => Token::Latin, C274 => Token::Latin, C275 => Token::Latin,
            C276 => Token::Latin, C277 => Token::Latin, C278 => Token::Latin, C279 => Token::Latin,
            C280 => Token::Latin, C281 => Token::Latin, C282 => Token::Latin, C283 => Token::Latin,
            C284 => Token::Latin, C285 => Token::Latin, C286 => Token::Latin, C287 => Token::Latin,
            C288 => Token::Latin, C289 => Token::Latin, C290 => Token::Latin, C291 => Token::Latin,
            C292 => Token::Latin, C293 => Token::Latin, C294 => Token::Latin, C295 => Token::Latin,
            C296 => Token::Latin, C297 => Token::Latin, C298 => Token::Latin, C299 => Token::Latin,
            C300 => Token::Latin, C301 => Token::Latin, C302 => Token::Latin, C303 => Token::Latin,
            C304 => Token::Latin, C305 => Token::Latin, C306 => Token::Latin, C307 => Token::Latin,
            C308 => Token::Latin, C309 => Token::Latin, C310 => Token::Latin, C311 => Token::Latin,
            C312 => Token::Latin, C313 => Token::Latin, C314 => Token::Latin, C315 => Token::Latin,
            C316 => Token::Latin, C317 => Token::Latin, C318 => Token::Latin, C319 => Token::Latin,
            C320 => Token::Latin, C321 => Token::Latin, C322 => Token::Latin, C323 => Token::Latin,
            C324 => Token::Latin, C325 => Token::Latin, C326 => Token::Latin, C327 => Token::Latin,
            C328 => Token::Latin, C329 => Token::Latin, C330 => Token::Latin, C331 => Token::Latin,
            C332 => Token::Latin, C333 => Token::Latin, C334 => Token::Latin, C335 => Token::Latin,
            C336 => Token::Latin, C337 => Token::Latin, C338 => Token::Latin, C339 => Token::Latin,
            C340 => Token::Latin, C341 => Token::Latin, C342 => Token::Latin, C343 => Token::Latin,
            C344 => Token::Latin, C345 => Token::Latin, C346 => Token::Latin, C347 => Token::Latin,
            C348 => Token::Latin, C349 => Token::Latin, C350 => Token::Latin, C351 => Token::Latin,
            C352 => Token::Latin, C353 => Token::Latin, C354 => Token::Latin, C355 => Token::Latin,
            C356 => Token::Latin, C357 => Token::Latin, C358 => Token::Latin, C359 => Token::Latin,
            C360 => Token::Latin, C361 => Token::Latin, C362 => Token::Latin, C363 => Token::Latin,
            C364 => Token::Latin, C365 => Token::Latin, C366 => Token::Latin, C367 => Token::Latin,
            C368 => Token::Latin, C369 => Token::Latin, C370 => Token::Latin, C371 => Token::Latin,
            C372 => Token::Latin, C373 => Token::Latin, C374 => Token::Latin, C375 => Token::Latin,
            C376 => Token::Latin, C377 => Token::Latin, C378 => Token::Latin, C379 => Token::Latin,
            C380 => Token::Latin, C381 => Token::Latin, C382 => Token::Latin, C383 => Token::Latin,
            C384 => Token::Latin, C385 => Token::Latin, C386 => Token::Latin, C387 => Token::Latin,
            C388 => Token::Latin, C389 => Token::Latin, C390 => Token::Latin, C391 => Token::Latin,
            C392 => Token::Latin, C393 => Token::Latin, C394 => Token::Latin, C395 => Token::Latin,
            C396 => Token::Latin, C397 => Token::Latin, C398 => Token::Latin, C399 => Token::Latin,
            C400 => Token::Latin, C401 => Token::Latin, C402 => Token::Latin, C403 => Token::Latin,
            C404 => Token::Latin, C405 => Token::Latin, C406 => Token::Latin, C407 => Token::Latin,
            C408 => Token::Latin, C409 => Token::Latin, C410 => Token::Latin, C411 => Token::Latin,
            C412 => Token::Latin, C413 => Token::Latin, C414 => Token::Latin, C415 => Token::Latin,
            C416 => Token::Latin, C417 => Token::Latin, C418 => Token::Latin, C419 => Token::Latin,
            C420 => Token::Latin, C421 => Token::Latin, C422 => Token::Latin, C423 => Token::Latin,
            C424 => Token::Latin, C425 => Token::Latin, C426 => Token::Latin, C427 => Token::Latin,
            C428 => Token::Latin, C429 => Token::Latin, C430 => Token::Latin, C431 => Token::Latin,
            C432 => Token::Latin, C433 => Token::Latin, C434 => Token::Latin, C435 => Token::Latin,
            C436 => Token::Latin, C437 => Token::Latin, C438 => Token::Latin, C439 => Token::Latin,
            C440 => Token::Latin, C441 => Token::Latin, C442 => Token::Latin, C443 => Token::Latin,
            C444 => Token::Latin, C445 => Token::Latin, C446 => Token::Latin, C447 => Token::Latin,
            C448 => Token::Latin, C449 => Token::Latin, C450 => Token::Latin, C451 => Token::Latin,
            C452 => Token::Latin, C453 => Token::Latin, C454 => Token::Latin, C455 => Token::Latin,
            C456 => Token::Latin, C457 => Token::Latin, C458 => Token::Latin, C459 => Token::Latin,
            C460 => Token::Latin, C461 => Token::Latin, C462 => Token::Latin, C463 => Token::Latin,
            C464 => Token::Latin, C465 => Token::Latin, C466 => Token::Latin, C467 => Token::Latin,
            C468 => Token::Latin, C469 => Token::Latin, C470 => Token::Latin, C471 => Token::Latin,
            C472 => Token::Latin, C473 => Token::Latin, C474 => Token::Latin, C475 => Token::Latin,
            C476 => Token::Latin, C477 => Token::Latin, C478 => Token::Latin, C479 => Token::Latin,
            C480 => Token::Latin, C481 => Token::Latin, C482 => Token::Latin, C483 => Token::Latin,
            C484 => Token::Latin, C485 => Token::Latin, C486 => Token::Latin, C487 => Token::Latin,
            C488 => Token::Latin, C489 => Token::Latin, C490 => Token::Latin, C491 => Token::Latin,
            C492 => Token::Latin, C493 => Token::Latin, C494 => Token::Latin, C495 => Token::Latin,
            C496 => Token::Latin, C497 => Token::Latin, C498 => Token::Latin, C499 => Token::Latin,
            C500 => Token::Latin, C501 => Token::Latin, C502 => Token::Latin, C503 => Token::Latin,
            C504 => Token::Latin, C505 => Token::Latin, C506 => Token::Latin, C507 => Token::Latin,
            C508 => Token::Latin, C509 => Token::Latin, C510 => Token::Latin, C511 => Token::Latin,
            _ => Token::Other,
        }
    }
}

fn check(input: &str, expected: Token) {
    let mut lexbuf = LexBuf::new(Utf8Refiller::new(String::from(input)));
    let tok = lex(&mut lexbuf);
    assert_eq!(
        tok, expected,
        "input {:?}: expected {:?}, got {:?}",
        input, expected, tok
    );
}

fn main() {
    // First partition: table value 1.
    check("\u{0100}", Token::Latin);
    // Middle of the table range.
    check("\u{0180}", Token::Latin);
    // Partition 254: table value 255 = u8::MAX.
    // A wrapping overflow would store 0 here, returning Other instead.
    check("\u{01FE}", Token::Latin);
    // Partition 255: excluded from the table by the i >= 255 guard,
    // classified via the decision tree instead.
    check("\u{01FF}", Token::Latin);
    // First of the 257 partitions handled by the decision tree.
    check("\u{0200}", Token::Latin);
    // Middle of the second batch.
    check("\u{0280}", Token::Latin);
    // Last pattern: U+02FF (partition 511).
    check("\u{02FF}", Token::Latin);
    // Outside the matched range.
    check("\u{0300}", Token::Other);
    check("A", Token::Other);

    println!("max_table: all cases passed");
}
