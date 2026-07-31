# Chancengleichheit der Spielmodi

Stand: 2026-07-30

## Verbindliche Grundregel

In einem kompetitiven Modus erhalten alle Spieler innerhalb derselben Runde
dieselben zufällig erzeugten Bedingungen. Dazu gehören insbesondere
Zielsegmente, Farben, Gefahren, Aufgaben, Karten und deren Reihenfolge.

- Zufall wird einmal pro Runde erzeugt, nicht beim Spielerwechsel.
- Eine Folge für Dart 1, 2 und 3 wird für jeden Spieler identisch wiederholt.
- Persönliche Zustände wie Combo, Heat, Ladung oder Fortschritt bleiben
  individuell.
- Eine neue Zufallsverteilung wird erst erzeugt, nachdem alle aktiven Spieler
  ihre Runde beendet haben.
- Abweichungen sind nur zulässig, wenn asymmetrische Rollen, eine gemeinsame
  fortlaufende Spielwelt oder direkte Spielerinteraktion ausdrücklich zum
  Spielprinzip gehören. Die Abweichung muss in der Anleitung genannt werden.
- Koop-Modi teilen Sieg und Sessionpunkte. Zufällige Ziele gehören dort zur
  gemeinsamen Welt und werden nicht als persönliche Chancen verteilt.

Neue Spielmodi müssen diese Entscheidung im Regelreview ausdrücklich
dokumentieren und mit einem Mehrspieler-Test absichern.

## Audit der vorhandenen Modi

| Kategorie | Modi | Bewertung |
| --- | --- | --- |
| Klassische symmetrische Regeln | Count Up, X01, Cricket, King of the Board | Kein persönlicher Zufall; gleiche Grundbedingungen. |
| Gemeinsames Rundenlayout | Avoid the Bomb, Color Clash, Dragon Eggs, Lightning Round, Mini Golf | Zufall wird pro Runde gemeinsam erzeugt. |
| Gemeinsame Aufgabenfolge | Cookie Monster, Target Rush, Ghost Chase, Simon Says | Jeder Spieler erhält dieselbe Board-, Ziel- beziehungsweise Sequenzfolge; persönlicher Fortschritt bleibt getrennt. |
| Gemeinsame Aufgabenkarte | Darts Bingo | Alle starten mit derselben Karte; nach dem ersten Bingo erhalten die übrigen Spieler derselben Runde eine Ausgleichschance. |
| Koop-Welt | Boss Fight, Block Drop, DartSweeper, Space Defender | Eine gemeinsam veränderte Welt; Team-Sieg und gleiche Sessionpunkte. |
| Bewusst asymmetrisch | Risk It, Heart Chase, Robin Hood Hunt, Eight Ball, Candy Cannon | Rollen, Vorgängerziel, feste Kugelgruppen oder Angriffe sind das erklärte Spielprinzip; kein verdeckt unterschiedlich verteilter Zufall. Bei Risk It erhält jeder Hot Pot genau eine Diebstahlchance durch den direkt folgenden Spieler. |
| Gemeinsame Fundwelt | Treasure Hunt | Alle greifen auf dieselbe verdeckte Karte zu. Das ist verständlich, erzeugt aber einen möglichen Startspieler- und Informationsvorteil und bleibt als Fairness-Thema offen. |

## Offener Punkt

`Treasure Hunt` benötigt vor einer endgültigen V1-Freigabe eine Entscheidung:

1. gemeinsame, konsumierbare Schatzkarte beibehalten und die Startreihenfolge
   zwischen Partien rotieren, oder
2. mathematisch gleichwertige persönliche Schatzkarten verwenden und
   aufgedeckte Inhalte nur für den aktiven Spieler anzeigen.

Variante 1 ist interaktiver und zuschauerfreundlicher. Variante 2 ist strenger
symmetrisch, benötigt aber eine klar getrennte Projektor-Darstellung.
