# BERICHT wiki-0013 (fs-wiki 0.0.13), Abnahme U1-U8 gegen fs-components 0.0.20

## Stand
- Pin in `crates/fs-wiki-app/Cargo.toml`: `tag = "v0.0.20"` (Tag existiert noch nicht). Geprüft wurde (neu) gegen Commit 3d25106 (editor-fixes; Bilder neu erzeugt), vorher 6c13308 (editor-fixes, Zweig unreviewed/editor-0020) über einen lokalen Pfad-Austausch der Abhängigkeiten (nicht committet; saubere Kopie mit `git archive` nach /srv/FreeSynergy/.scratch/comp-6c13308). Cargo.lock ist deshalb NICHT neu aufgelöst (nur fs-wiki-Versionen 0.0.13); nach dem Tag: `cargo update -p fs-components` bzw. Lock neu, dann CI.
- Gates lokal gegen 6c13308: fmt ok, clippy -D warnings ok, `cargo test --workspace --all-features --locked` ok (alle).

## Abnahme (Bilder: /srv/FreeSynergy/.scratch/wiki-0013-bilder/, Test tests/window_acceptance.rs, Notion-Frontend, Fensterstile, Seite des Nutzers)
- U1/U2: u1-u2-kaestchen.png: Kästchen in Liste und Tabellenzelle quadratisch ~18 px, leer, in der Textzeile; Test setzt und hebt auf (Liste x2, Zelle) über den Router. Nicht nachgestellt: ein echter Mausklick im echten Fenster.
- U3: u3-schreibflaeche.png / u3-nach-enter.png: keine Kästen je Block, eine Fläche mit rohem Markdown, Titel eigenes Feld. Test: Enter bei Block 1 macht einen Block mehr, Fokus geht weiter, Gang durch alle Blöcke mit Zeichnen verliert kein Zeichen. Eindruck: Enter mitten in "## mal sehen" trennte "## ma" / "l sehen" (richtig, aber im Bild sieht man den Markdown roh, kein WYSIWYG, Grenze in 0.0.20).
- U4/U5: u4-u6-breite-1900.png: Seiten-Icon ganz, unter der Kopfzeile; Gliederungsmarke rechts (Strich bei x~1870) ganz.
- U6: u4-u6-breite-1900.png, u6-breite-1300.png, u6-volle-breite-1900.png: Brotkrume und Titel beginnen am selben x (726 bei 1900 px); Spalte ~660 px breit, mittig; volle Breite beginnt neben der Seitenleiste (x 288), Brotkrume mit.
- U7: u7-u8-tabelle-code.png: Tabellentext so hoch wie Absatz (Test: gleich hoch, 11 px).
- U8: Code-Block hat Rahmen, eine Zeile (~50 px statt 81). EHRLICH: der Hintergrund ist im Bild NICHT sichtbar, Grund = Token --surface, im Notion-Stil weiß (255,255,255) wie die Seite. Bedarf an fs-components/Stil: Grund dort abgesetzt (z.B. Notion-Grau).

## Angepasste Tests (Folge von 0.0.20)
- editor_width: Editor ohne Rahmen, Breite am umbrochenen Absatz (>= 500 px, gemessen 577 bei Spalte 732).
- window_tasks: Tabelle 4 statt 3 Textzeilen (lange Wörter brechen bei Fließtextgröße um).
- tree_after_save: Gegenprobe "ohne Haken bleibt der Baum" entfernt: 0.0.20 macht die Seitenliste nach dem Speichern auch ohne Haken neu. Haken bleibt. Strategie: prüfen, ob er überflüssig ist.
- fmt-Nacharbeit an Commit 40d7f51 (frontend_window.rs, window_*).

## Offen
- Tag v0.0.20 abwarten, Pin/Lock/CI, Push-Stand prüfen.
- Echter Fenster-Handtest (Mausklick, Tastatur) fehlt; Abnahme ist im Prozess.
- U8 Hintergrund (siehe oben).
