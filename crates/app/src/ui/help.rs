//! The help page: a short manual in German and English, its language switchable on the page.

use egui::{Rect, RichText, Ui, UiBuilder};
use mr_sim::settings::Lang;

use super::theme::*;
use super::{App, View};
use crate::store;

/// Lines get hard to read wider than this.
const TEXT_W: f32 = 760.0;
/// Width of the first column of a table, the keys.
const KEY_W: f32 = 210.0;

pub(super) enum B {
    H(&'static str),
    P(&'static str),
    Steps(&'static [&'static str]),
    Grid(&'static [(&'static str, &'static str)]),
    /// Settings, high scores and recording: the labels, the paths are where they are
    /// on this system.
    Files([&'static str; 3]),
}

use B::*;

mod more;

const DE: &[B] = &[
    H("Überblick"),
    P("MORSEHUNTER ist ein CW-Contest-Simulator. Du rufst CQ, Stationen antworten, und du nimmst sie nach Gehör auf, durch Rauschen, Fading und Störungen hindurch. Das Programm setzt Morse Runner 1.68 von VE3NEA neu in Rust um, mit eigenem Fenster und Eingabe per Paddle."),
    H("Erste Schritte"),
    Steps(&[
        "Unter Optionen dein Rufzeichen eintragen, Tempo und Tonhöhe einstellen.",
        "F9 oder Alt-R startet ein Pile-Up, F10 hält den Lauf an.",
        "Die Knöpfe über dem Log starten jeden Modus.",
    ]),
    P("Tut F9 nichts, hat der Desktop die Taste für sich belegt. Dann Alt-R nehmen."),
    H("Modi"),
    Grid(&[
        ("Pile-Up", "Auf jeden CQ antwortet eine zufällige Zahl von Stationen. Übt das Heraushören."),
        ("Einzelanrufe", "Immer eine Station, gleich nach dem letzten QSO. Übt das Tippen."),
        ("WPX-Wettbewerb", "Wie Pile-Up, aber mit festen Bedingungen und fester Dauer. Die Punkte kommen in die Bestenliste und zählen für den Rekord."),
        ("HST-Wettbewerb", "Regeln der IARU-Schnelltelegrafie. Schnelle Anrufer, keine Bandeffekte, die Punkte richten sich nach der Länge der Rufzeichen."),
    ]),
    P("Nach einem Wettbewerb kommen deine eigenen Einstellungen zurück."),
    P("Die Nummern der Anrufer zählen wie im Original ab dem Start des Laufs, je nach Können mit höchstens ein bis drei QSOs pro Minute. Am Anfang liegen ihre Nummern darum dicht bei deinen. Unter Optionen im Feld Bandbedingungen stellt Contest läuft seit die Stunden ein, die der Contest schon vor deinem Start lief. Bei 6 h kommen meist Nummern bis etwa 1000, so wie beim späten Einstieg in einen echten Contest. Wettbewerbe beginnen immer bei 0."),
    P("Die Anrufer geben wie im Original zwischen der Hälfte deines Tempos und deinem Tempo, bei 15 wpm also 8 bis 15 wpm. Unter Optionen im Feld Tempo und Ton stellen Anrufer ab und Anrufer bis den Bereich in Prozent deines Tempos ein, für plus minus 10 % etwa 90 und 110. Darunter steht, was das bei deinem Tempo in wpm ergibt. Änderst du dein Tempo im Lauf, behalten die Stationen, die schon rufen, ihr Tempo. Erst neue Anrufer richten sich nach dem neuen. Im HST-Wettbewerb geben alle genau dein Tempo, Wettbewerbe nehmen immer 50 und 100 %."),
    P("Aktivität schwankt im Feld Bandbedingungen lässt die Anrufer in Wellen kommen, mit ruhigen Phasen und etwa alle 20 Minuten einer kurzen Öffnung, in der doppelt so viele rufen. Das gibt es im Original nicht. Es ist ab Werk aus, Wettbewerbe laufen immer ohne, ein Lauf-Code nimmt es mit."),
    H("Ein QSO mit der Enter-Taste"),
    Steps(&[
        "Rufzeichenfeld leer, Enter ruft CQ.",
        "Gehörtes Rufzeichen tippen, Enter sendet sein Rufzeichen und deinen Report.",
        "Seine Nummer tippen, Enter sendet TU und trägt das QSO ins Log.",
    ]),
    P("Drückst du Enter, bevor seine Nummer im Feld steht, fragt das Programm mit ? nach. Das zählt als Rückfrage."),
    H("Teilrufzeichen mit ?"),
    P("Ein ? im Rufzeichen markiert eine Stelle, die du nicht aufgenommen hast. Die Stationen verstehen W1? oder ?BC als Teil eines Rufzeichens. Steht ein ? im Feld, sendet Enter nur dieses Teilrufzeichen, ohne Report. Wer sich gemeint fühlt, gibt sein Rufzeichen noch einmal, alle anderen warten still, bis dein QSO zu Ende ist."),
    Steps(&[
        "XX? tippen, Enter sendet nur XX?.",
        "Er antwortet mit DE XX9HM.",
        "Das ? durch 9HM ersetzen, Enter sendet XX9HM und deinen Report.",
        "Er gibt seinen Report, seine Nummer tippen, Enter sendet TU.",
    ]),
    P("Ersetze nur das ? und lösche das Feld nicht ganz. Ein leeres Rufzeichenfeld setzt das QSO zurück. Mit nur zwei Buchstaben fühlen sich manchmal mehrere Stationen gemeint, mit mehr Zeichen wird es eindeutiger. Ein ? im Rufzeichen zählt nicht als Rückfrage."),
    P("Änderst du ein Rufzeichen, während es hinausgeht, folgt der Rest der Nachricht, solange der schon gesendete Teil noch passt."),
    H("Tasten"),
    Grid(&[
        ("F1 bis F8", "CQ, Report, TU, Mein Call, Sein Call, QSO B4, ?, NIL"),
        ("Enter", "CQ, dann Rufzeichen und Report, dann TU und Log"),
        ("Leertaste", "599 einsetzen, zwischen Rufzeichen und Nummer springen"),
        ("Tab, Umschalt-Tab", "nächstes oder vorheriges Feld"),
        ("Esc", "Senden abbrechen"),
        ("; oder Einfg", "sein Rufzeichen und Report, mit ? im Rufzeichen nur das Rufzeichen"),
        ("+ . , [", "TU und Log"),
        ("\\", "CQ"),
        ("F11, Alt-W", "Felder leeren"),
        ("Alt-Enter, Umschalt-Enter", "ins Log, ohne zu senden"),
        ("Pfeil hoch, runter", "RIT"),
        ("Alt-Pfeil hoch, runter", "Bandbreite"),
        ("Bild auf, Bild ab", "Tempo in Schritten von 5 wpm, auch Alt-F10 und Alt-F9"),
        ("F9, Alt-R", "Pile-Up starten"),
        ("F10", "Lauf anhalten"),
    ]),
    P("Die Strg-Varianten des Originals gehen ebenso, außer Strg gibt gerade das Paddle."),
    P("Über dem Filterstreifen rechts neben den Feldern zeigt ein kleiner Wasserfall das Band vor dem Filter, ±800 Hz um deine Tonhöhe. Die neueste Zeile liegt unten am Streifen. Die beiden Striche sind die Ränder des Filters. Ruft jemand neben der Frequenz, siehst du seine Spur, bevor du ihn hörst. Ein Klick auf die Spur stellt das RIT auf ihn. Ein Klick in den Streifen links oder rechts vom Fenster verschiebt das RIT um 50 Hz, ein Klick ins Fenster setzt es auf null. Dein eigenes Signal zeigt der Wasserfall nicht."),
    H("Von Hand geben"),
    P("Unter Optionen im Feld Paddle das Häkchen bei Von Hand geben setzen. Ein VBand-USB-Adapter läuft ohne Treiber. Er meldet sich als Tastatur, die Paddles liegen auf linker und rechter Strg-Taste. Jedes andere Tastenpaar lässt sich dort einstellen. Es gibt Iambic A und B, Ultimatic, Bug und Handtaste."),
    P("Das Programm dekodiert, was du gibst, und sagt den Stationen, was es gelesen hat. Ein falsches Rufzeichen bekommt eine Korrektur, Unleserliches bringt nichts. Das Programm lernt deinen Abstand zwischen Buchstaben mit. Als Wortlücke gilt erst eine Pause von gut dem 1,6-fachen, und ein Durchgang endet nach Stille von etwa 3,6 Buchstabenabständen, höchstens 1,5 s, bei langsamem Tempo nach 12 Punktlängen. Am Anfang, solange es dich noch nicht kennt, wartet es eine volle Sekunde. An Handtaste und Bug lernt es Punkt- und Strichlänge getrennt. So passt es auch, wenn der Keyer schneller ist als dein Rhythmus zwischen den Buchstaben. Rufzeichen und Nummer tippst du wie gewohnt in die Felder. Ein gegebenes TU trägt das QSO ein, sobald die Felder vollständig sind. Wer lieber mit Alt-Enter loggt, schaltet das ab."),
    P("Das Paddle unterbricht eine Nachricht von der Tastatur, eine Funktionstaste beendet den gegebenen Durchgang."),
    P("Unter Optionen im Feld Tempo und Ton schaltet QSK das Mithören zwischen den Zeichen, ab Werk ist es an. Mit QSK geht der Empfänger in jeder Pause zwischen Punkten und Strichen auf. So hörst du, wenn eine Station in deine Sendung hineinruft. Ohne QSK ist der Empfänger stumm, solange du sendest, ob per Taste oder Paddle. Ein Lauf-Code legt QSK mit fest."),
    H("Vail-Adapter"),
    P("Der Vail-Adapter (github.com/Vail-CW/vail-adapter) meldet sich als Tastatur und als MIDI-Gerät. Das Punkt-Paddle kommt als linke Strg-Taste, das Strich-Paddle als rechte, also passt die Voreinstellung des Programms. Ab Werk tastet er aber mit eigenem Keyer, Iambic B bei etwa 12 wpm. Hängt der Keyer des Programms dahinter, tasten zwei Keyer hintereinander und die Buchstaben zerfallen, aus CQ wird TEATTA. Darum stellst du den Adapter auf Passthrough. Dann meldet er nur, welches Paddle gerade gedrückt ist, und Tempo, Keyer und Mithörton kommen aus dem Programm."),
    Steps(&[
        "Adapter einstecken. Unter Optionen im Feld Paddle Von Hand geben setzen, unter Vail-Adapter den Adapter wählen (unter Linux QT Py M0 MIDI 1, er steht meist schon oben) und auf Passthrough klicken. Er speichert das und bleibt auch nach dem Abstecken dabei.",
        "Dit bleibt ControlLeft, Dah bleibt ControlRight. Den Keyer wählen, meist Iambic B.",
        "Das Programmfenster braucht den Fokus, sonst landen die Strg-Tasten in einem anderen Fenster.",
    ]),
    P("Ohne den Knopf geht es unter Linux mit amidi -l und amidi -p hw:2,0,0 -S 'C0 00'. Passthrough braucht Firmware 5.1 oder neuer. Ältere Firmware sendet darin gar keine Tasten mehr. Dann mit Eigener Keyer zurück auf Iambic B im Adapter, das stellt den Keyer hier auf Handtaste. Der Keyer im Adapter gibt dann Tempo und Abstände vor, und das Programm liest nur mit. Das Tempo stellst du dann am Adapter ein. Besser ist es, die Firmware zu erneuern, das steht in der README."),
    P("Im Passthrough schweigt der Summer des Adapters, du hörst den Mithörton des Programms. Liegen Punkt und Strich verkehrt, hilft Paddles tauschen in den Optionen. Gib nie KSKS als eigenes Wort. Das schaltet den Adapter in den Tastatur-Modus, in dem er selbst dekodiert und Buchstaben tippt. Abstecken und wieder einstecken beendet das."),
    H("Lauf-Code"),
    P("Jeder Lauf hat einen Code wie MR-0HZA-Y5QN-7K85-W7X5-XC. Er steht unter den Knöpfen über dem Log und im Ergebnisstreifen, ein Klick kopiert ihn. Unter Optionen bei Geteilter Lauf eingegeben oder mit morsehunter --code gestartet, ergibt er denselben Lauf. Dieselben Stationen kommen in derselben Reihenfolge, mit demselben Rauschen und denselben Störungen. Tempo, Tempo der Anrufer, Tonhöhe, Bandbreite, QSK, Bandbedingungen, Aktivität, wie lange der Contest schon läuft, Laufzeit und Regionen bleiben bis zum Ende fest."),
    P("Ändert sich während eines Laufs eine Bedingung, verschwindet sein Code. RIT zählt nicht. Rufzeichen, Name, Mithörton, Sprache, Aufnahme und Paddle gehören nicht zum Code."),
    H("Log-Markierungen"),
    Grid(&[
        ("NIL", "nicht im anderen Log, falsches Rufzeichen, oder er hat deins nie richtig bekommen"),
        ("DUP", "schon gearbeitet"),
        ("RST", "falscher Report"),
        ("NR", "falsche Nummer"),
    ]),
    P("Die Spalte Land zeigt das Land des geloggten Rufzeichens, bestimmt aus seinem Präfix nach cty.dat, in der Sprache des Fensters. Mit der Maus darauf steht der volle Name da."),
    H("Contest-Punkte"),
    P("QSOs mal WPX-Präfixe, roh aus deinem Log und geprüft gegen die Logs der Stationen. Im HST richten sich die Punkte nach der Länge der Rufzeichen."),
    H("Leistungspunkte"),
    P("Jeder Lauf, der von selbst endet, bekommt Leistungspunkte, auch ein Übungslauf. Sie gibt es im Original nicht."),
    Grid(&[
        ("Gutes QSO", "10 Punkte bei 25 wpm, im Verhältnis zum Tempo, bei 35 wpm also 14"),
        ("Rückfrage", "?, NR? oder AGN, per Taste oder Paddle, kostet je 20 % des QSO-Werts, mindestens 40 % bleiben"),
        ("Fehlerhaftes QSO", "NIL, RST oder NR, zieht die Hälfte seines Werts ab, in Lst steht es also negativ"),
        ("Dupe", "zählt nichts"),
        ("Bandeffekt", "je 10 % mehr für QRN, QRM, QSB, Flutter und LIDs"),
        ("Contest läuft seit", "ab 1 h 10 % mehr, ab 6 h 20 %, für die längeren Nummern"),
    ]),
    P("Tempo und Bedingungen zählen so, wie sie beim Eintragen des QSOs waren. Im Log zeigt Rf die Rückfragen und Lst die Punkte jedes QSOs, Summe und Hochrechnung pro Stunde stehen über dem Log rechts. Verglichen wird nach Punkten pro Stunde, damit kurze und lange Läufe zusammenpassen."),
    H("Bestenliste"),
    P("Vier Listen. Leistung führt jeden Lauf nach Punkten pro Stunde, dazu kommen WPX, HST und per Code gestartete Pile-Up- oder Einzelanruf-Läufe. Ein Lauf kommt hinein, wenn er von selbst endet, danach öffnet sich die Liste mit dem neuen Eintrag. Wer mit F10 stoppt, bekommt keine Wertung, außer im HST. Ein Übungslauf ohne gutes QSO bleibt draußen. Ein Klick auf eine Zeile kopiert ihren Code."),
    P("Verlauf zeigt die besten Leistungspunkte pro Stunde jedes Tages als Kurve. Mit der Maus auf einem Punkt stehen Tag, Wert und Zahl der Läufe da."),
    H("Auswertung"),
    P("Der Reiter Auswertung neben dem Log zeigt, was im Lauf schiefging, auch während er läuft. Jedes fehlerhafte QSO steht da mit deiner Eingabe und dem, was die Gegenstation gab. Bei einem falschen Rufzeichen sucht das Programm den Anrufer mit dem ähnlichsten Rufzeichen. Darunter steht, welche Zeichen du verwechselt, ausgelassen oder zu viel geschrieben hast, die häufigsten zuerst."),
    P("Gibst du mit dem Paddle, misst Gebeweise deine Abstände in Punkten deines eingestellten Tempos, neben dem Sollwert. Mit Keyer sind das die Pausen zwischen Buchstaben (Soll 3) und Wörtern (Soll 7), denn Punkte und Striche macht der Keyer. Mit Handtaste oder Bug kommen Punkt, Strich, ihr Verhältnis (Soll 3) und die Gleichmäßigkeit dazu. Was auffällt, steht als Hinweis darunter. Ab 2,4 Punkten Buchstabenabstand kann der Decoder die Buchstaben trennen, darunter laufen sie ineinander."),
    H("Gesendet und Debug"),
    P("Unter Optionen im Feld Anzeige lässt sich ein Reiter Gesendet neben dem Log einschalten. Er listet alles, was gesendet wird, mit der Zeit im Lauf. Deine Nachrichten, was die Anrufer geben, und QRM. Was gerade läuft, ist hell. Der Reiter verrät jedes Rufzeichen. Ein Lauf, in dem er irgendwann an war, bekommt darum keine Punkte und keinen Platz in der Bestenliste, oben rechts steht dann Keine Wertung."),
    P("Im selben Feld wählst du unter Farben das Aussehen. Morsehunter dunkel ist das gewohnte Bild, Morsehunter hell dasselbe Gerät bei Tag. DL8UG hell und DL8UG dunkel tragen die Farben von dl8ug.de und seinen PDFs. Hoher Kontrast nimmt nur Schwarz, Weiß und Gelb, für schwache Augen. Der Wechsel gilt sofort, auch mitten im Lauf."),
    H("Dateien"),
    Files(["Einstellungen", "Bestenliste", "Letzte Aufnahme"]),
];

const EN: &[B] = &[
    H("Overview"),
    P("MORSEHUNTER is a CW contest simulator. You call CQ, stations answer, and you copy them by ear through noise, fading and interference. It is a new implementation in Rust of Morse Runner 1.68 by VE3NEA, with its own window and paddle input."),
    H("Getting started"),
    Steps(&[
        "Enter your call under Options, set speed and pitch.",
        "F9 or Alt-R starts a pile-up, F10 stops the run.",
        "The buttons above the log start every mode.",
    ]),
    P("If F9 does nothing, the desktop keeps that key for itself. Use Alt-R."),
    H("Modes"),
    Grid(&[
        ("Pile-Up", "A random number of stations answers each CQ. Trains your copying."),
        ("Single calls", "One station at a time, right after the last contact. Trains your typing."),
        ("WPX competition", "Like Pile-Up with fixed conditions and time. The score goes into the high scores and counts for your record."),
        ("HST competition", "The rules of the IARU high speed telegraphy events. Fast callers, no band effects, the score follows the length of the calls."),
    ]),
    P("Your own settings come back after a competition."),
    P("As in the original, the callers' numbers count from the start of the run, at most one to three contacts a minute by their skill, so at first their numbers lie close to yours. Contest running for, in the Band conditions card of the options, sets the hours the contest was on before you joined. At 6 h most numbers go up to about 1000, as when you join a real contest late. Competitions always start at 0."),
    P("As in the original, callers send between half your speed and your speed, so 8 to 15 wpm at 15 wpm. Callers from and Callers up to, in the Speed and sound card of the options, set the range in percent of your speed, for plus or minus 10 % say 90 and 110. Below them stands what that gives at your speed in wpm. When you change your speed during a run, the stations already calling keep theirs, only new callers follow. In the HST competition everybody sends exactly your speed, competitions always take 50 and 100 %."),
    P("Activity comes in waves, in the Band conditions card, lets callers come in waves, with lulls and about every 20 minutes a short opening in which twice as many call. The original has no such thing. It is off by default, competitions always run without it, a run code takes it along."),
    H("A contact with the Enter key"),
    Steps(&[
        "Call field empty, Enter calls CQ.",
        "Type the call you heard, Enter sends his call and your exchange.",
        "Type his number, Enter sends TU and logs the contact.",
    ]),
    P("Press Enter before his number is in the field and the program asks with ?. That counts as a question."),
    H("Partial calls with ?"),
    P("A ? in the call marks a place you did not copy, the stations take W1? or ?BC as a partial call. With a ? in the field, Enter sends only that partial call, without the exchange. Whoever feels meant gives his call again, all others wait quietly until your contact is over."),
    Steps(&[
        "Type XX?, Enter sends only XX?.",
        "He answers DE XX9HM.",
        "Replace the ? with 9HM, Enter sends XX9HM and your exchange.",
        "He gives his exchange, type his number, Enter sends TU.",
    ]),
    P("Replace only the ?, do not clear the field. An empty call field starts the contact over. With only two letters several stations may feel meant, more characters make it clear. A ? in the call does not count as a question."),
    P("Change a call while it goes out and the rest of the message follows, as long as the part already sent still fits."),
    H("Keys"),
    Grid(&[
        ("F1 to F8", "CQ, exchange, TU, my call, his call, QSO B4, ?, NIL"),
        ("Enter", "CQ, then call and exchange, then TU and log"),
        ("Space", "fill in 599, jump between call and number"),
        ("Tab, Shift-Tab", "next or previous field"),
        ("Esc", "stop sending"),
        ("; or Insert", "his call and exchange, with a ? in the call only the call"),
        ("+ . , [", "TU and log"),
        ("\\", "CQ"),
        ("F11, Alt-W", "wipe the fields"),
        ("Alt-Enter, Shift-Enter", "log without sending"),
        ("Up, Down", "RIT"),
        ("Alt-Up, Alt-Down", "bandwidth"),
        ("PgUp, PgDn", "speed in steps of 5 wpm, also Alt-F10 and Alt-F9"),
        ("F9, Alt-R", "start a pile-up"),
        ("F10", "stop the run"),
    ]),
    P("The Ctrl variants of the original work as well, unless Ctrl keys the paddle."),
    P("Above the filter strip to the right of the fields, a small waterfall shows the band before the filter, ±800 Hz around your pitch. The newest line is at the bottom, next to the strip. The two lines mark the edges of the filter. When someone calls off frequency, you see the trace before you hear the station. Click the trace and the RIT tunes to it. A click in the strip left or right of the window moves the RIT by 50 Hz, a click in the window sets it to zero. Your own signal does not show in the waterfall."),
    H("Keying by hand"),
    P("Tick Key by hand in the Paddle card of the options. A VBand USB paddle adapter works without a driver, it shows up as a keyboard whose paddles are left and right Ctrl. Any other pair of keys can be set there. Iambic A and B, ultimatic, bug and straight key are available."),
    P("The program decodes what you send and tells the stations what it read, so a wrong call gets a correction and noise gets you nothing. The program learns your pause between letters. Only a pause about 1.6 times as long counts as a word gap, and an over ends after a silence of about 3.6 letter gaps, 1.5 s at most, or 12 dits at slow speeds. Until it knows you, it waits a full second. On a straight key or bug it learns dit and dah lengths apart. This works even when the keyer is faster than your rhythm between letters. Type the call and the number you copied into the fields as usual. A keyed TU logs the contact when the fields are complete. Turn that off if you prefer to log with Alt-Enter."),
    P("A paddle stops a message from the keyboard, a function key finishes the keyed over."),
    P("QSK in the Speed and sound card of the options lets you hear the band between your elements, it is on by default. With QSK the receiver opens in every gap between dits and dahs, so you hear a station that calls into your sending. Without QSK the receiver is silent while you send, by key or by paddle. A run code fixes QSK as well."),
    H("Vail adapter"),
    P("The Vail adapter (github.com/Vail-CW/vail-adapter) shows up as a keyboard and as a MIDI device. The dit paddle comes as left Ctrl and the dah paddle as right Ctrl, so the program's defaults fit. Out of the box, though, it keys with its own keyer, iambic B at about 12 wpm. With the program's keyer behind it, two keyers run in series and the letters fall apart, CQ turns into TEATTA. So set the adapter to passthrough. Then it only reports which paddle is pressed, and speed, keyer and sidetone come from the program."),
    Steps(&[
        "Plug the adapter in. Tick Key by hand in the Paddle card of the options, pick the adapter under Vail adapter (QT Py M0 MIDI 1 on Linux, it is usually on top already) and click Passthrough. The adapter stores it and keeps it after a replug.",
        "Leave Dit on ControlLeft and Dah on ControlRight. Pick the keyer, usually iambic B.",
        "The program window needs the focus, or the Ctrl keys end up in another window.",
    ]),
    P("Without the button, on Linux, amidi -l and amidi -p hw:2,0,0 -S 'C0 00' do the same. Passthrough needs firmware 5.1 or newer. Older firmware sends no keys at all in it. Then Its own keyer goes back to the adapter's iambic B and sets the keyer here to Straight key. The adapter's keyer then sets speed and spacing and the program only reads along, so you set the speed on the adapter. Updating the firmware is better, the README tells how."),
    P("In passthrough the adapter's buzzer stays quiet and you hear the program's sidetone. If dit and dah are the wrong way round, use Swap paddles in the options. Never key KSKS as a word of its own. It puts the adapter in keyboard sim mode, where it decodes by itself and types letters. Unplugging it ends that."),
    H("Run code"),
    P("Every run has a code like MR-0HZA-Y5QN-7K85-W7X5-XC. It stands below the buttons above the log and in the result strip, a click copies it. Enter it under Shared run in the options, or start with morsehunter --code, and you get the same run. The same stations come in the same order, the same noise and interference. Speed, the callers' speeds, pitch, bandwidth, QSK, band conditions, activity, how long the contest has been on, run length and regions stay fixed until the end."),
    P("If a condition changes during a run, its code disappears. RIT does not count. Your call, name, sidetone, language, recording and paddle are not part of the code."),
    H("Log marks"),
    Grid(&[
        ("NIL", "not in the other log, wrong call, or he never got yours right"),
        ("DUP", "worked before"),
        ("RST", "wrong report"),
        ("NR", "wrong serial number"),
    ]),
    P("The Country column shows the country of the logged call, from its prefix as in cty.dat, in the language of the window. Hover over it for the full name."),
    H("Contest score"),
    P("QSOs times WPX prefixes, raw from your log and verified against the logs of the stations. In HST the points follow the length of the calls."),
    H("Skill points"),
    P("Every run that ends by itself gets skill points, practice runs too. The original has none."),
    Grid(&[
        ("Good contact", "10 points at 25 wpm, in proportion to the speed, so 14 at 35 wpm"),
        ("Question", "?, NR? or AGN, by key or paddle, takes 20 % of the contact each, 40 % always stay"),
        ("Busted contact", "NIL, RST or NR, takes off half its worth, so Skl shows it negative"),
        ("Dupe", "counts nothing"),
        ("Band effect", "10 % more each for QRN, QRM, QSB, flutter and LIDs"),
        ("Contest running for", "10 % more from 1 h on, 20 % from 6 h on, for the longer numbers"),
    ]),
    P("Speed and conditions count as they were when the contact went into the log. In the log, the ? column shows the questions and Skl the points of each contact, the sum and the rate per hour stand above the log, on the right. Runs compare by points per hour, so short and long runs fit together."),
    H("High scores"),
    P("Four lists. Skill ranks every run by points per hour, then WPX, HST, and pile-up or single calls runs started from a code. A run gets in when it ends by itself, then the list opens with the new line marked. A run stopped with F10 scores nothing, except in HST. A practice run without a good contact stays out. A click on a line copies its code."),
    P("Progress shows the best skill points per hour of each day as a curve. Hover over a dot for the day, the value and the number of runs."),
    H("Analysis"),
    P("The Analysis tab beside the log shows what went wrong in the run, while it runs as well. Every contact that is not clean stands there with what you logged and what the other side sent. For a wrong call the program looks for the caller with the closest call. Below it stands which characters you mixed up, left out or wrote too many, the most frequent first."),
    P("When you key with the paddle, Keying measures your gaps in dits of your set speed, next to the book value. With a keyer those are the pauses between letters (book 3) and words (book 7), since the keyer makes the dits and dahs. With a straight key or bug the dit, the dah, their ratio (book 3) and the steadiness come along. What stands out is named below. From 2.4 dits between letters the decoder can tell them apart, below that they run together."),
    H("Sent and debug"),
    P("The Display card of the options turns on a tab Sent beside the log. It lists everything that goes out with the time in the run. Your messages, what the callers key, and QRM. What is on the air right now is lit. The tab gives away every call, so a run with it on at any time scores nothing and stays out of the high scores. The header then reads No score."),
    P("In the same card, Colours picks the look. Morsehunter dark is the familiar one, Morsehunter light the same set by day. DL8UG light and DL8UG dark wear the colours of dl8ug.de and its PDFs. High contrast uses only black, white and yellow, for weak eyes. The change takes effect at once, even during a run."),
    H("Files"),
    Files(["Settings", "High scores", "Last recording"]),
];

impl App {
    pub(super) fn help(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let lang = self.help_lang.unwrap_or(v.settings.lang);
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            egui::ScrollArea::vertical().id_salt("help").auto_shrink([false, false]).show(ui, |ui| {
                let w = (ui.available_width() - 14.0).min(TEXT_W);
                ui.set_max_width(w);
                ui.horizontal_wrapped(|ui| {
                    for l in Lang::ALL {
                        if ui.selectable_label(lang == l, l.name()).clicked() {
                            self.help_lang = Some(l);
                        }
                    }
                });
                let blocks = match lang {
                    Lang::En => EN,
                    Lang::De => DE,
                    Lang::Fr => more::FR,
                    Lang::Es => more::ES,
                    Lang::Pt => more::PT,
                    Lang::It => more::IT,
                    Lang::Pl => more::PL,
                    Lang::Ru => more::RU,
                };
                for b in blocks {
                    show(ui, b);
                }
                ui.add_space(20.0);
            });
        });
    }
}

fn show(ui: &mut Ui, b: &B) {
    let body = |t: &str| RichText::new(t).size(14.5).color(pal().text);
    match b {
        H(t) => {
            ui.add_space(14.0);
            ui.label(RichText::new(*t).size(18.0).color(pal().glow));
            ui.add_space(2.0);
        }
        P(t) => {
            ui.label(body(t));
            ui.add_space(4.0);
        }
        Steps(list) => {
            for (n, t) in list.iter().enumerate() {
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new(format!("{}.", n + 1)).monospace().size(14.0).color(pal().glow_dim));
                    // in a row a label only wraps when told to
                    ui.add(egui::Label::new(body(t)).wrap());
                });
            }
            ui.add_space(4.0);
        }
        Files(labels) => {
            let home = std::env::var("HOME").unwrap_or_default();
            let paths = [store::config_path(), store::scores_path(), store::wav_path()];
            for (k, p) in labels.iter().zip(paths) {
                let p = p.map(|p| p.display().to_string()).unwrap_or_default();
                let p = match p.strip_prefix(&home) {
                    Some(rest) if !home.is_empty() => format!("~{rest}"),
                    _ => p,
                };
                key_row(ui, k, |ui| {
                    ui.add(egui::Label::new(body(&p)).wrap());
                });
            }
            ui.add_space(4.0);
        }
        Grid(rows) => {
            // a fixed first column, the second one wraps
            for (k, t) in rows.iter() {
                key_row(ui, k, |ui| {
                    ui.add(egui::Label::new(body(t)).wrap());
                });
            }
            ui.add_space(4.0);
        }
    }
}

/// A line of a table: the key in a fixed column, then what wraps.
fn key_row(ui: &mut Ui, key: &str, rest: impl FnOnce(&mut Ui)) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(KEY_W, 18.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.set_min_width(KEY_W);
            ui.label(RichText::new(key).monospace().size(13.5).color(pal().glow_dim));
        });
        rest(ui);
    });
    ui.add_space(2.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(blocks: &[B]) -> Vec<(u8, usize)> {
        blocks
            .iter()
            .map(|b| match b {
                H(_) => (0, 0),
                P(_) => (1, 0),
                Steps(l) => (2, l.len()),
                Grid(r) => (3, r.len()),
                Files(_) => (4, 3),
            })
            .collect()
    }

    /// Each translation follows the English manual block for block.
    #[test]
    fn every_language_has_the_whole_manual() {
        let en = shape(EN);
        for (name, blocks) in [("de", DE), ("fr", more::FR), ("es", more::ES), ("pt", more::PT), ("it", more::IT), ("pl", more::PL), ("ru", more::RU)] {
            assert_eq!(shape(blocks), en, "{name}");
        }
    }
}
