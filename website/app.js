document.documentElement.classList.add("js");

const SITE_LANGUAGE_KEY = "sdb-website-language";
const SITE_EN = {
  "Smart Dartboard — Eine echte Scheibe. Eine ganze Spielhalle.": "Smart Dartboard — One real board. A whole arcade.",
  "Smart Dartboard verwandelt eine echte Dartscheibe mit Projektor, Touch-Steuerung und modularen Spielmodi in eine kleine Arcade.": "Smart Dartboard turns a real dartboard into a compact arcade with projection, touch controls, and modular game modes.",
  "Zum Inhalt springen": "Skip to content", "Navigation öffnen": "Open navigation", "Hauptnavigation": "Main navigation",
  "Smart Dartboard Startseite": "Smart Dartboard home", "Produkteigenschaften": "Product features", "Projektumfang": "Project scope",
  "Erlebnis": "Experience", "Statistiken": "Statistics", "Spielmodi": "Game modes", "Technik": "Technology", "Sprache wählen": "Choose language",
  "DARTS, ABER ARCADE.": "DARTS, TURNED ARCADE.", "Eine echte Scheibe.": "One real board.", "Eine ganze Spielhalle.": "A whole arcade.",
  "Smart Dartboard legt digitale Spielwelten exakt über ein echtes Board. Wählen, werfen, jubeln – mit Projektor, Touch-Controller, Sound und Spielmodi, die sich jedes Mal anders anfühlen.": "Smart Dartboard maps digital game worlds precisely onto a real board. Choose, throw, celebrate—with a projector, touch controller, sound, and game modes that feel different every time.",
  "Gameplay ansehen": "See gameplay", "Projekt auf GitHub": "View on GitHub", "Lokaler Betrieb": "Local operation", "ECHTE TESTSESSION": "REAL TEST SESSION",
  "Spielmodi": "game modes", "spezialisierte Screens": "specialized screens", "echte Dartscheibe": "real dartboard", "modular erweiterbar": "modular by design",
  "VON NULL AUF SPIELBETRIEB": "FROM ZERO TO GAME ON", "Einfach genug für den ersten Wurf.": "Simple from the first throw.", "Tief genug für den ganzen Abend.": "Deep enough for the whole night.",
  "Die Bedienung bleibt auf dem Controller. Der Projektor gehört vollständig dem Spiel.": "Controls stay on the controller. The projector belongs entirely to the game.",
  "01 · AUSWÄHLEN": "01 · CHOOSE", "Spieler, Modus und Optionen in wenigen Berührungen": "Players, mode, and options in a few taps",
  "Session starten": "Start a session", "Spielerprofile antippen. Ergebnisse und Siege laufen über mehrere Games weiter.": "Tap player profiles. Results and wins carry across multiple games.",
  "Modus wählen": "Choose a mode", "Optionen erklären ihre konkrete Regel live auf Controller und Projektor – ohne verschachtelte Menüs.": "Options explain their exact rule live on controller and projector—without nested menus.",
  "Loswerfen": "Start throwing", "Board-Overlays zeigen Ziele und Gefahren. Sound bestätigt Treffer, Wechsel und Sieg.": "Board overlays show targets and dangers. Sound confirms hits, turns, and wins.",
  "PIXELGENAU": "PIXEL PRECISE", "Die Projektion sitzt auf der echten Scheibe.": "Projection aligned to the real board.",
  "Vierpunkt-Kalibrierung, runder Reset und eine dauerhaft mittige Board-Geometrie halten digitale Ziele dort, wo der Dart landet.": "Four-point calibration, a circular reset, and permanently centered board geometry keep digital targets exactly where the dart lands.",
  "HÖRBAR DIREKT": "INSTANT AUDIO", "Sounds ohne zweiten Blick.": "Feedback without looking away.",
  "Treffer, Miss, Countdown, Spielerwechsel und Sieg bekommen klare akustische Signale über den Projektor.": "Hits, misses, countdowns, player changes, and wins get clear audio cues through the projector.",
  "Ein Abend, viele Spiele.": "One night, many games.", "Gewonnene Games geben Sessionpunkte. Dauerhafte Profile sammeln Ergebnisse, Trefferbilder und persönliche Bestwerte über den Abend hinaus.": "Won games award session points. Persistent profiles collect results, hit patterns, and personal bests beyond the current night.",
  "DEIN ABEND. DEINE WÜRFE.": "YOUR NIGHT. YOUR THROWS.", "Jedes Spiel bleibt": "Every game stays", "nachvollziehbar.": "traceable.",
  "Statistiken, Heatmaps und vollständige Replays entstehen lokal – ohne Benutzerkonto und ohne Cloud.": "Statistics, heatmaps, and complete replays are created locally—without an account or cloud.",
  "STATISTIK & SPIELVERLAUF": "STATS & GAME HISTORY", "Siege, 3-Dart-Average, Trefferbilder und abgeschlossene Games.": "Wins, three-dart average, hit patterns, and completed games.",
  "WURF FÜR WURF": "THROW BY THROW", "Ein Replay hält Boardzustand, Punkte und Korrekturen fest.": "A replay preserves board state, scores, and corrections.",
  "PRIVAT BY DESIGN": "PRIVATE BY DESIGN", "Deine Daten bleiben in der Spielhalle.": "Your data stays in the arcade.",
  "Nur beendete Games zählen in die Wertung. Testspiele lassen sich getrennt einblenden und die komplette Historie kann als JSON exportiert werden.": "Only completed games count. Test games can be included separately, and the complete history can be exported as JSON.",
  "Sprache sofort wechseln": "Switch language instantly", "Flaggen im Controller schalten die gesamte Oberfläche um. Der Projektor folgt automatisch.": "Flags on the controller switch the entire interface. The projector follows automatically.",
  "Würfe direkt korrigieren": "Correct throws directly", "Treffer auswählen, auf der Korrekturscheibe neu setzen, als Miss markieren oder einen fehlenden Wurf nachtragen.": "Select a hit, place it again on the correction board, mark it as a miss, or add a missing throw.",
  "Keine Cloud notwendig": "No cloud required", "Sessions, Spiele und das unveränderliche Ereignisjournal liegen ausschließlich in der lokalen SQLite-Datenbank.": "Sessions, games, and the immutable event journal live exclusively in the local SQLite database.",
  "ECHTES GAMEPLAY": "REAL GAMEPLAY", "Die Scheibe wird zum": "The board becomes a", "Spielbrett.": "game board.",
  "Alle Aufnahmen stammen automatisiert aus einer lokalen WebKit-Testsession.": "Every screenshot was captured automatically from a local WebKit test session.",
  "Darts steuern gemeinsam einen Block-Puzzler.": "Darts control a shared block puzzler.", "Laden, Risiko nehmen, Bull treffen.": "Charge, take the risk, hit Bull.",
  "Punkte sammeln – rote Felder meiden.": "Score points—avoid red segments.", "Eier bergen – die dritte Schuppe entfacht das Feuer.": "Collect eggs—the third scale ignites dragon fire.",
  "24 MODI. EINE SCHEIBE.": "24 MODES. ONE BOARD.", "Jeder Wurf kann etwas": "Every throw can mean", "anderes bedeuten.": "something different.",
  "Klassiker, Party-Games, Koop-Abenteuer und Arcade-Challenges teilen sich dieselbe präzise Trefferlogik.": "Classics, party games, co-op adventures, and arcade challenges share the same precise hit detection.",
  "Aufladen, nicht überhitzen und mit Bull feuern.": "Charge, avoid overheating, and fire with Bull.", "Bewegen, drehen und gemeinsam Linien bauen.": "Move, rotate, and clear lines together.",
  "Felder aufdecken und Minen logisch umspielen.": "Reveal fields and reason around mines.", "Schwachpunkte finden und gemeinsam Schaden machen.": "Find weak points and deal damage together.",
  "Wellen stoppen, bevor die Flotte zu groß wird.": "Stop waves before the fleet grows too large.", "Ein gemeinsamer Kurs, möglichst wenige Darts.": "One shared course, as few darts as possible.",
  "Eigene Zahlen räumen, dann Double Bull versenken.": "Clear your numbers, then sink Double Bull.", "Goldene Eier bergen, rote Schuppen und Drachenfeuer meiden.": "Collect golden eggs; avoid red scales and dragon fire.",
  "ZWEI ARTWORK-PACKS": "TWO ARTWORK PACKS", "Deine Spielhalle.": "Your arcade.", "Dein Look.": "Your style.",
  "Wechsle im Board-Setup zwischen warmer 3D-Spielzeugwelt und klassischer Neon-Arcade.": "Switch between a warm 3D toy world and classic neon arcade in board setup.",
  "Warm · charmant · familienfreundlich": "Warm · charming · family-friendly", "Dunkel · filmisch · intensiv": "Dark · cinematic · intense",
  "ECHTE HARDWARE, KLARE ROLLEN": "REAL HARDWARE, CLEAR ROLES", "Drei Teile.": "Three parts.", "Ein flüssiges Spiel.": "One seamless game.",
  "Die Scheibe liefert Treffer, der Controller führt durch den Abend, der Projektor macht jeden Wurf sichtbar und hörbar.": "The board delivers hits, the controller guides the night, and the projector makes every throw visible and audible.",
  "Treffer kommen seriell und entprellt ins Spiel.": "Hits enter the game serially and debounced.", "Touch-Controller": "Touch controller",
  "Spieler, Modi, aktuelle Würfe, Touch-Korrektur, Abbruch und Setup.": "Players, modes, current throws, touch correction, abort, and setup.",
  "Projektor": "Projector", "Kalibrierte Ziele, Spielstand, Animation und Sound.": "Calibrated targets, game state, animation, and sound.",
  "DARTBOARD-KOMPATIBILITÄT": "DARTBOARD COMPATIBILITY", "Ein Board ist getestet.": "One board is tested.", "Das System bleibt offen.": "The system stays open.",
  "Die aktuelle Installation läuft mit echter Hardware. Weitere Trefferquellen können über einen passenden Decoder oder Eingabe-Adapter angebunden werden.": "The current installation runs with real hardware. Additional hit sources can be connected through a suitable decoder or input adapter.",
  "PRAKTISCH GETESTET": "TESTED IN PRACTICE", "Verwendetes Modell ansehen": "View tested model", "ERWEITERBAR": "EXTENSIBLE", "Weitere elektronische Boards": "Other electronic boards",
  "Art. 3663107 · Bluetooth 4.0 · meldet sich als SDB-BT. Mit diesem Softdartboard wird das Projekt entwickelt und im realen Spielbetrieb getestet.": "Art. 3663107 · Bluetooth 4.0 · advertised as SDB-BT. This soft-tip dartboard is used to develop and test the project in real operation.",
  "Grundsätzlich anbindbar, sofern Treffer elektronisch ausgelesen werden können. Jedes noch nicht getestete Protokoll benötigt einen eigenen Decoder oder Adapter.": "Generally compatible when hits can be read electronically. Every untested protocol needs its own decoder or adapter.",
  "MIT TREFFERERKENNUNG": "WITH HIT DETECTION", "Klassische Sisal-Scheiben": "Classic sisal boards",
  "Die Projektion lässt sich geometrisch ausrichten. Für automatische Treffer braucht eine klassische Scheibe zusätzlich eine Kamera-, Sensor- oder andere Eingabelösung.": "Projection can be aligned geometrically. Automatic hit detection on a classic board additionally needs a camera, sensor, or another input solution.",
  "FÜR NEUE IDEEN GEBAUT": "BUILT FOR NEW IDEAS", "Spielmodi sind Module.": "Game modes are modules.", "Nicht Core-Code.": "Not core code.",
  "Jeder Modus bringt Regeln, Optionen, Anleitungen und Board-Overlays mit. Der gemeinsame Core kümmert sich um BLE, Sessions, Undo, Screens und Persistenz.": "Every mode brings its own rules, options, instructions, and board overlays. The shared core handles BLE, sessions, undo, screens, and persistence.",
  "Architektur auf GitHub ansehen": "View architecture on GitHub", "Vier Schritte.": "Four steps.", "Dann fliegen die Darts.": "Then the darts fly.",
  "Das veröffentlichte Image läuft auf AMD64 und ARM64. Docker genügt für Oberfläche und Testtreffer; echte BLE-Hardware hängt vom Host ab.": "The published image runs on AMD64 and ARM64. Docker is enough for the UI and test hits; real BLE hardware depends on the host.",
  "Repository holen": "Clone the repository", "Enthält Compose-Datei und sichere Standardwerte.": "Includes the Compose file and secure defaults.",
  "Release laden": "Pull the release", "Version 0.0.2 wird reproduzierbar aus GHCR geladen.": "Version 0.0.2 is pulled reproducibly from GHCR.",
  "Container starten": "Start the container", "Der erste Start läuft ohne BLE und erlaubt Testtreffer.": "The first start runs without BLE and allows test hits.",
  "Healthcheck prüfen": "Check health", "Danach Controller und Projektor im Browser öffnen.": "Then open controller and projector in the browser.",
  "Befehle kopieren": "Copy commands", "Danach öffnen": "Then open", "Auf anderen Geräten": "On other devices", "durch die IP des Docker-Rechners ersetzen.": "replace it with the Docker host's IP address.",
  "Für den festen Spielhallenbetrieb auf Linux:": "For permanent arcade operation on Linux:", "die BLE-Adresse eintragen,": "set the BLE address,", "setzen. Der Host benötigt BlueZ und einen erreichbaren": "The host needs BlueZ and an accessible", "-Socket.": "socket.",
  "Nativ mit BLE": "Native with BLE", "verbindet unterstützte Boards direkt. Unter macOS wird Bluetooth über CoreBluetooth freigegeben.": "connects supported boards directly. On macOS, Bluetooth is provided through CoreBluetooth.",
  "UI und Testbetrieb": "UI and test operation", "Auf macOS und Windows läuft der Container ohne BLE. Projektor-Klicks und Testtreffer stehen für Einrichtung und Demo bereit.": "On macOS and Windows, the container runs without BLE. Projector clicks and test hits are available for setup and demos.",
  "Container mit Hardware": "Container with hardware", "Mit Host-Netzwerk, BlueZ und DBus kann auch der Produktionscontainer das BLE-Dartboard direkt erreichen.": "With host networking, BlueZ, and DBus, the production container can directly reach the BLE dartboard.",
  "READY TO THROW?": "READY TO THROW?", "Bring Arcade-Energie": "Bring arcade energy", "auf die echte Scheibe.": "to the real board.",
  "Release 0.0.2, Setup-Anleitung und der komplette Code liegen auf GitHub.": "Release 0.0.2, setup instructions, and the complete source are on GitHub.",
  "Release 0.0.2 ansehen": "View release 0.0.2", "Asset-Lizenz": "Asset license", "Nach oben ↑": "Back to top ↑",
  "Deutsch": "German", "Bild schließen": "Close image", "Vergrößerte Gameplay-Aufnahme": "Enlarged gameplay screenshot",
  "Spielauswahl vergrößern": "Enlarge game selection", "Statistikansicht vergrößern": "Enlarge statistics view", "Replayansicht vergrößern": "Enlarge replay view",
  "Artwork-Theme": "Artwork theme", "Systemdiagramm": "System diagram", "Beispiel eines Spielmodus-Moduls": "Example game-mode module", "Docker-Quickstart-Befehle kopieren": "Copy Docker quickstart commands",
  "Candy Cannon auf dem Projektor: Das Bull ist als FIRE-Ziel markiert.": "Candy Cannon on the projector: Bull is marked as the FIRE target.",
  "Touch-Controller während einer Candy-Cannon-Runde.": "Touch controller during a Candy Cannon round.",
  "Grafische Spielauswahl für drei Spieler mit großen Moduskarten.": "Graphical game selection for three players with large mode cards.",
  "Lokale Statistikansicht mit Spielerwerten, Treffer-Heatmap, Sessions und Spielmodi.": "Local statistics view with player values, hit heatmap, sessions, and game modes.",
  "Replay eines abgeschlossenen Count-Up-Spiels mit Board, Punktestand und Wurfleiste.": "Replay of a completed Count Up game with board, score, and throw strip.",
  "Block Drop Darts mit vier farbigen Steuerflächen und Blockraster.": "Block Drop Darts with four colored control areas and a block grid.",
  "Candy Cannon mit aufgeladenem Bull als Feuer-Trigger.": "Candy Cannon with a charged Bull as the fire trigger.",
  "Avoid the Bomb mit roten Bombenfeldern auf dem Dartboard.": "Avoid the Bomb with red bomb segments on the dartboard.",
  "Dragon Eggs mit roten Schuppenfeldern und einer großen Drachenfeuer-Reaktion.": "Dragon Eggs with red scale segments and a large dragon-fire reaction.",
  "Count Up im hellen Playful-Cartoon-Theme.": "Count Up in the bright Playful Cartoon theme.",
  "Count Up im dunklen Classic-Neon-Theme.": "Count Up in the dark Classic Neon theme."
};

function preferredSiteLanguage(){
  try{
    const saved=localStorage.getItem(SITE_LANGUAGE_KEY);
    if(saved==='de'||saved==='en') return saved;
  }catch(_){ /* Storage can be unavailable in privacy mode. */ }
  return String(navigator.language||'').toLowerCase().startsWith('de')?'de':'en';
}

const siteLanguage=preferredSiteLanguage();
document.documentElement.lang=siteLanguage;

function translateSite(){
  document.querySelectorAll('[data-site-language]').forEach(button=>{
    button.classList.toggle('active',button.dataset.siteLanguage===siteLanguage);
    button.setAttribute('aria-pressed',String(button.dataset.siteLanguage===siteLanguage));
  });
  if(siteLanguage!=='en') return;
  const quickstartNote=document.querySelector('.quickstart-note span');
  if(quickstartNote){
    quickstartNote.innerHTML='<b>For permanent arcade operation on Linux:</b> In <code>.env</code>, set the BLE address, <code>SDB_ENABLE_BLE=1</code>, and <code>SDB_ALLOW_TEST_EVENTS=0</code>. The host needs BlueZ and an accessible <code>/var/run/dbus</code> socket.';
  }
  const testedBoardCopy=document.querySelector('.compatibility-card h3 + p');
  if(testedBoardCopy){
    testedBoardCopy.innerHTML='Art. 3663107 · Bluetooth 4.0 · advertised as <code>SDB-BT</code>. This soft-tip dartboard is used to develop and test the project in real operation.';
  }
  const walker=document.createTreeWalker(document.body,NodeFilter.SHOW_TEXT);
  const nodes=[];
  while(walker.nextNode()) nodes.push(walker.currentNode);
  nodes.forEach(node=>{
    if(node.parentElement?.closest('script,style,pre,code')) return;
    const raw=node.nodeValue||'';
    const value=raw.trim().replace(/\s+/g,' ');
    if(SITE_EN[value]){
      const leading=raw.match(/^\s*/)?.[0]||'';
      const trailing=raw.match(/\s*$/)?.[0]||'';
      node.nodeValue=leading+SITE_EN[value]+trailing;
    }
  });
  document.querySelectorAll('[aria-label],[title],[alt],[content]').forEach(element=>{
    for(const attr of ['aria-label','title','alt','content']){
      const value=element.getAttribute(attr);
      if(value&&SITE_EN[value]) element.setAttribute(attr,SITE_EN[value]);
    }
  });
  if(SITE_EN[document.title]) document.title=SITE_EN[document.title];
}

translateSite();

document.querySelectorAll('[data-site-language]').forEach(button=>{
  button.addEventListener('click',()=>{
    try{localStorage.setItem(SITE_LANGUAGE_KEY,button.dataset.siteLanguage);}catch(_){ /* ignore */ }
    location.reload();
  });
});

const header = document.querySelector("[data-header]");
const nav = document.querySelector("[data-nav]");
const navToggle = document.querySelector("[data-nav-toggle]");

function updateHeader() {
  header?.classList.toggle("scrolled", window.scrollY > 24);
}

updateHeader();
window.addEventListener("scroll", updateHeader, { passive: true });

navToggle?.addEventListener("click", () => {
  const open = !nav.classList.contains("open");
  nav.classList.toggle("open", open);
  navToggle.setAttribute("aria-expanded", String(open));
});

nav?.addEventListener("click", event => {
  if (event.target.closest("a")) {
    nav.classList.remove("open");
    navToggle?.setAttribute("aria-expanded", "false");
  }
});

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const revealItems = document.querySelectorAll(".reveal");

if (reducedMotion || !("IntersectionObserver" in window)) {
  revealItems.forEach(item => item.classList.add("visible"));
} else {
  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (!entry.isIntersecting) return;
      entry.target.style.setProperty(
        "--delay",
        `${Number(entry.target.dataset.delay || 0)}ms`,
      );
      entry.target.classList.add("visible");
      observer.unobserve(entry.target);
    });
  }, { threshold: 0.12 });
  revealItems.forEach(item => observer.observe(item));
}

const themeStage = document.querySelector("[data-theme-stage]");
const themeName = themeStage?.querySelector("[data-theme-name]");

themeStage?.addEventListener("click", event => {
  const button = event.target.closest("[data-theme-button]");
  if (!button) return;
  const theme = button.dataset.themeButton;

  themeStage.querySelectorAll("[data-theme-button]").forEach(candidate => {
    const selected = candidate === button;
    candidate.classList.toggle("active", selected);
    candidate.setAttribute("aria-selected", String(selected));
  });
  themeStage.querySelectorAll("[data-theme-image]").forEach(image => {
    image.classList.toggle("active", image.dataset.themeImage === theme);
  });
  if (themeName) {
    themeName.textContent = theme === "neon" ? "CLASSIC NEON" : "PLAYFUL CARTOON";
  }
});

const lightbox = document.querySelector("[data-lightbox]");
const lightboxImage = lightbox?.querySelector("img");

document.addEventListener("click", event => {
  const trigger = event.target.closest("[data-zoom]");
  if (!trigger || !lightbox || !lightboxImage) return;
  const source = trigger.dataset.zoom;
  const thumbnail = trigger.querySelector("img");
  lightboxImage.src = source;
  lightboxImage.alt = thumbnail?.alt || "Vergrößerte Gameplay-Aufnahme";
  lightbox.showModal();
});

lightbox?.addEventListener("click", event => {
  if (event.target === lightbox || event.target.closest("[data-lightbox-close]")) {
    lightbox.close();
  }
});

document.addEventListener("keydown", event => {
  if (event.key === "Escape" && lightbox?.open) {
    lightbox.close();
  }
});

const quickstartCopyButton = document.querySelector("[data-copy-quickstart]");
const quickstartCode = document.querySelector("[data-quickstart-code]");

async function copyQuickstart() {
  if (!quickstartCopyButton || !quickstartCode) return;
  const commands = quickstartCode.textContent.trim();

  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(commands);
    } else {
      const textarea = document.createElement("textarea");
      textarea.value = commands;
      textarea.setAttribute("readonly", "");
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.append(textarea);
      textarea.select();
      const copied = document.execCommand("copy");
      textarea.remove();
      if (!copied) throw new Error("Copy command was rejected");
    }

    quickstartCopyButton.classList.add("copied");
    quickstartCopyButton.innerHTML = `<span aria-hidden="true">✓</span> ${siteLanguage==='en'?'Copied':'Kopiert'}`;
    window.setTimeout(() => {
      quickstartCopyButton.classList.remove("copied");
      quickstartCopyButton.innerHTML = `<span aria-hidden="true">□</span> ${siteLanguage==='en'?'Copy commands':'Befehle kopieren'}`;
    }, 2200);
  } catch {
    quickstartCopyButton.textContent = siteLanguage==='en'?'Please select manually':'Bitte manuell markieren';
  }
}

quickstartCopyButton?.addEventListener("click", copyQuickstart);
