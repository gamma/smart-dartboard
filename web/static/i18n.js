(function(){
  'use strict';

  const ui = {
    de: {
      ready_when_you_are:'BEREIT, WENN IHR ES SEID',
      hero_title:'Eure Darts. Eure Session.',
      hero_copy:'Spieler auswählen, Spielmodus antippen und loslegen.',
      start_session:'Session starten',
      calibrate:'Projektor kalibrieren',
      statistics:'Statistiken',
      step_players:'SCHRITT 1 VON 2',
      who_plays:'Wer spielt heute?',
      choose_players:'Wählt bis zu acht Spieler für diese Session.',
      create_first_player:'Legt euren ersten Spieler an.',
      new_player:'Neuer Spieler',
      name:'Name',
      player_name:'Spielername',
      avatar:'Avatar',
      color:'Farbe',
      create_player:'Spieler anlegen',
      back:'Zurück',
      selected:'Ausgewählt',
      tap:'Antippen',
      players_selected:'{count} Spieler gewählt',
      continue_games:'Weiter zur Spielauswahl',
      german:'Deutsch',
      english:'English',
      open_projector:'Projektor öffnen',
      step_games:'SCHRITT 2 VON 2',
      choose_game:'Wählt euer Spiel',
      modes_ready:'{count} Spieler · alle Modi sind sofort startklar.',
      end_session:'Session beenden',
      other_game:'Anderer Spielmodus',
      start_game:'Spiel starten',
      starter:'STARTSPIELER',
      choose_starter:'Startspieler wählen',
      starts_game:'{name} beginnt',
      random_starter:'Zufall',
      starter_rotation:'Automatische Rotation',
      starter_manual:'Manuell gewählt',
      starter_random:'Zufällig ausgelost',
      starter_copy:'Nach jedem gewerteten Spiel startet automatisch der nächste Spieler. Abgebrochene Spiele verändern die Reihenfolge nicht.',
      projector_running:'PROJEKTOR LÄUFT',
      game_starts:'Spiel startet …',
      line_ready:'Alle Spieler bereit an die Linie.',
      current_task:'AKTUELLE AUFGABE',
      round:'RUNDE',
      turn_finished:'Aufnahme beendet',
      player_turn:'{name} ist dran',
      darts:'DARTS',
      points_short:'PKT',
      continue_player:'Weiter zum nächsten Spieler',
      undo:'Letzte Aktion zurück',
      abort_game:'Spiel abbrechen',
      early_end:'Aufnahme vorzeitig beenden',
      skip_confirm:'Restliche Darts wirklich überspringen?',
      switch_player:'Spieler wechseln',
      keep_playing:'Weiterspielen',
      abort_warning:'Dieses Spiel wird nicht gewertet.',
      confirm_abort:'Abbruch bestätigen',
      session_score:'SESSION-WERTUNG · 3 PUNKTE PRO SIEG',
      game_complete:'SPIEL BEENDET',
      back_to_games:'Zurück zur Spielauswahl',
      session_complete:'SESSION BEENDET',
      highlights:'Eure Highlights',
      saved:'Alle Ergebnisse wurden dauerhaft gespeichert.',
      home:'Zur Startseite',
      wins:'Siege',
      games:'Spiele',
      win_rate:'Siegquote',
      session_points:'Sessionpunkte',
      three_dart_average:'3-Dart-Average',
      stats_title:'Statistiken & Spielverlauf',
      stats_copy:'Sessions, Spiele, Trefferbilder und Trainingshinweise.',
      overview:'Übersicht',
      sessions:'Sessions',
      players:'Spieler',
      game_modes:'Spielmodi',
      heatmap:'Heatmap',
      training:'Training',
      heatmap_training:'Heatmap & Trainingshinweise',
      show_analysis:'Analyse anzeigen',
      analysis_active:'Analyse ausgewählt',
      replay:'Replay',
      no_data:'Noch keine abgeschlossenen Produktionsspiele.',
      test_data:'Testdaten einbeziehen',
      production_only:'Nur echte Spiele',
      close:'Schließen',
      all_players:'Alle Spieler',
      all_modes:'Alle Spielmodi',
      completed_games:'Gewertete Spiele',
      throws:'Würfe',
      hits:'Treffer',
      success_rate:'Erfolgsquote',
      completion_rate:'Abschlussquote',
      details:'Details',
      previous:'Zurück',
      next:'Weiter',
      event:'Ereignis',
      score:'Spielstand',
      recommendation:'Trainingsempfehlung',
      insufficient_data:'Noch zu wenig Zieldaten – startet mit diesen Grundlagen.',
      language_de:'DE',
      language_en:'EN',
      ready:'BEREIT',
      pull_darts:'DARTS ZIEHEN',
      final_score:'Finaler Spielstand',
      result:'ENDERGEBNIS',
      test_mode:'TESTMODUS',
      click_segment:'Scheibensegment anklicken',
      correction_hint:'Belegten Wurf zum Ändern oder den nächsten freien Platz zum Nachtragen antippen.',
      correct_throw:'WURF {number} KORRIGIEREN',
      add_throw_number:'WURF {number} NACHTRAGEN',
      tap_actual_segment:'Tatsächliches Feld antippen',
      correction_copy:'Wählt den tatsächlich getroffenen Bereich auf der Scheibe. Der Spielstand und alle folgenden Würfe werden neu berechnet.',
      board_input_paused:'Board-Eingabe während der Korrektur pausiert',
      count_as_miss:'Als MISS werten',
      delete_throw:'Wurf löschen',
      tap_to_edit:'ANTIPPEN ZUM ÄNDERN',
      add_throw:'Wurf nachtragen',
      open_slot:'NICHT GEWORFEN',
      current_turn:'Aktueller Zug',
      cancel:'Abbrechen',
      team_win:'TEAM-SIEG',
      together_done:'Gemeinsam geschafft!',
      for_everyone:'+3 FÜR ALLE',
      game_decided:'SPIEL ENTSCHIEDEN',
      game_won:'Spiel gewonnen',
      session_points_award:'+3 SESSIONSPUNKTE',
      draw:'GLEICHSTAND',
      draw_title:'Unentschieden',
      no_session_points:'KEINE SESSIONSPUNKTE',
      challenge_lost:'CHALLENGE VERLOREN',
      boss_wins:'Boss gewinnt',
      press_again:'NOCH EINMAL DRÜCKEN',
      double_press:'2× SPIELERWECHSEL DRÜCKEN',
      rematch_confirm:'Revanche wird bestätigt',
      same_game_rotates:'Gleiches Spiel · Startspieler wechselt',
      board_button:'SCHEIBEN-TASTE',
      rematch:'REVANCHE',
      calibrate_title:'Projektor ausrichten',
      calibrate_copy:'Verschiebt die vier Eckpunkte, bis der äußere Ring exakt auf der echten Scheibe liegt.',
      one_time_setup:'EINMALIGES SETUP',
      reset_center:'Rund und mittig zurücksetzen',
      save_calibration:'Kalibrierung speichern',
      calibration_projector:'Äußeren Ring am Control-Screen deckungsgleich ausrichten',
      overheat:'ÜBERHITZT!',
      that_was_close:'DAS WAR KNAPP!',
      charge_lost:'LADUNG VERLOREN',
      export_data:'Daten exportieren',
      session_control:'SESSION-STEUERUNG',
      board_error:'BOARD-FEHLER',
      board_searching:'BOARD WIRD GESUCHT',
      controls:'STEUERUNG',
      top_left:'Oben links',
      top_right:'Oben rechts',
      bottom_right:'Unten rechts',
      bottom_left:'Unten links',
      calibration_note:'Gemeldete Projektorfläche: {width} × {height}. „Rund und mittig“ setzt eine unverzerrte quadratische Fläche mit 5 % Sicherheitsrand auf der kürzeren Browserseite. Danach könnt ihr die vier Ecken fein auf die echte Scheibe legen.',
      artwork_theme:'ARTWORK-THEME',
      artwork_copy:'Neue Modi ohne altes Neon-Cover verwenden automatisch das Cartoon-Artwork.',
      projector_sound:'PROJEKTOR-SOUND',
      enabled:'Eingeschaltet',
      disabled:'Ausgeschaltet',
      status:'Status',
      status_active:'Aktiv',
      status_running:'Läuft',
      status_finished:'Beendet',
      status_aborted:'Abgebrochen',
      status_interrupted:'Unterbrochen',
      event_type_game_started:'Spiel gestartet',
      event_type_game_finished:'Spiel beendet',
      event_type_game_aborted:'Spiel abgebrochen',
      event_type_throw:'Wurf',
      event_type_throw_corrected:'Wurf korrigiert',
      event_type_throw_deleted:'Wurf gelöscht',
      event_type_continue_turn:'Aufnahme fortgesetzt',
      event_type_next_player:'Spielerwechsel',
      event_type_game_action:'Spielaktion',
      event_type_undo:'Korrektur zurückgenommen',
      environment_production:'Echtes Spiel',
      environment_test:'Testspiel',
      ruleset_version:'Regelversion {version}',
      ring_single:'Single',
      ring_double:'Double',
      ring_triple:'Triple',
      ring_single_bull:'Bull',
      ring_double_bull:'Doppel-Bull',
      ring_miss:'Daneben',
      outcome_success:'Treffer',
      outcome_partial:'Fast getroffen',
      outcome_danger:'Strafwurf',
      outcome_miss:'Daneben',
      outcome_neutral:'Neutral',
      sound_on:'Sound einschalten',
      sound_off:'Sound ausschalten',
      test_tone:'Testton',
      autoplay_help:'Der Projektor-Browser blockiert Autoplay. Im Kioskmodus die Autoplay-Freigabe aktivieren und die Projektorseite neu laden.',
      session_setup:'SESSION-SETUP',
      team_ready:'TEAM BEREIT',
    },
    en: {
      ready_when_you_are:'READY WHEN YOU ARE',
      hero_title:'Your darts. Your session.',
      hero_copy:'Choose players, tap a game mode, and start.',
      start_session:'Start session',
      calibrate:'Calibrate projector',
      statistics:'Statistics',
      step_players:'STEP 1 OF 2',
      who_plays:'Who is playing?',
      choose_players:'Choose up to eight players for this session.',
      create_first_player:'Create your first player.',
      new_player:'New player',
      name:'Name',
      player_name:'Player name',
      avatar:'Avatar',
      color:'Color',
      create_player:'Create player',
      back:'Back',
      selected:'Selected',
      tap:'Tap to select',
      players_selected:'{count} players selected',
      continue_games:'Continue to game selection',
      german:'Deutsch',
      english:'English',
      open_projector:'Open projector',
      step_games:'STEP 2 OF 2',
      choose_game:'Choose your game',
      modes_ready:'{count} players · all modes are ready to play.',
      end_session:'End session',
      other_game:'Choose another game',
      start_game:'Start game',
      starter:'STARTING PLAYER',
      choose_starter:'Choose starting player',
      starts_game:'{name} starts',
      random_starter:'Random',
      starter_rotation:'Automatic rotation',
      starter_manual:'Selected manually',
      starter_random:'Randomly drawn',
      starter_copy:'The next player automatically starts after every scored game. Aborted games do not change the order.',
      projector_running:'PROJECTOR READY',
      game_starts:'Game starts …',
      line_ready:'All players to the oche.',
      current_task:'CURRENT TARGET',
      round:'ROUND',
      turn_finished:'Visit complete',
      player_turn:'{name} to throw',
      darts:'DARTS',
      points_short:'PTS',
      continue_player:'Next player',
      undo:'Undo last action',
      abort_game:'Abort game',
      early_end:'End visit early',
      skip_confirm:'Skip the remaining darts?',
      switch_player:'Switch player',
      keep_playing:'Keep playing',
      abort_warning:'This game will not count.',
      confirm_abort:'Confirm abort',
      session_score:'SESSION STANDINGS · 3 POINTS PER WIN',
      game_complete:'GAME COMPLETE',
      back_to_games:'Back to game selection',
      session_complete:'SESSION COMPLETE',
      highlights:'Your highlights',
      saved:'All results have been saved.',
      home:'Back to start',
      wins:'Wins',
      games:'Games',
      win_rate:'Win rate',
      session_points:'Session points',
      three_dart_average:'3-dart average',
      stats_title:'Statistics & game history',
      stats_copy:'Sessions, games, hit maps, and training insights.',
      overview:'Overview',
      sessions:'Sessions',
      players:'Players',
      game_modes:'Game modes',
      heatmap:'Heatmap',
      training:'Training',
      heatmap_training:'Heatmap & training insights',
      show_analysis:'Show analysis',
      analysis_active:'Analysis selected',
      replay:'Replay',
      no_data:'No completed production games yet.',
      test_data:'Include test data',
      production_only:'Production games only',
      close:'Close',
      all_players:'All players',
      all_modes:'All game modes',
      completed_games:'Scored games',
      throws:'Throws',
      hits:'Hits',
      success_rate:'Success rate',
      completion_rate:'Completion rate',
      details:'Details',
      previous:'Previous',
      next:'Next',
      event:'Event',
      score:'Score',
      recommendation:'Training recommendation',
      insufficient_data:'Not enough target data yet – start with these fundamentals.',
      language_de:'DE',
      language_en:'EN',
      ready:'READY',
      pull_darts:'PULL DARTS',
      final_score:'Final score',
      result:'RESULT',
      test_mode:'TEST MODE',
      click_segment:'Click a dartboard segment',
      correction_hint:'Tap a recorded dart to edit it or the next open slot to add a missing throw.',
      correct_throw:'CORRECT DART {number}',
      add_throw_number:'ADD DART {number}',
      tap_actual_segment:'Tap the actual segment',
      correction_copy:'Tap the segment that was actually hit. The score and every following dart will be recalculated.',
      board_input_paused:'Board input paused while editing',
      count_as_miss:'Count as MISS',
      delete_throw:'Delete dart',
      tap_to_edit:'TAP TO EDIT',
      add_throw:'Add missing dart',
      open_slot:'NOT THROWN',
      current_turn:'Current turn',
      cancel:'Cancel',
      team_win:'TEAM WIN',
      together_done:'Completed together!',
      for_everyone:'+3 FOR EVERYONE',
      game_decided:'GAME DECIDED',
      game_won:'Game won',
      session_points_award:'+3 SESSION POINTS',
      draw:'DRAW',
      draw_title:'Draw',
      no_session_points:'NO SESSION POINTS',
      challenge_lost:'CHALLENGE LOST',
      boss_wins:'Boss wins',
      press_again:'PRESS ONCE MORE',
      double_press:'PRESS PLAYER SWITCH TWICE',
      rematch_confirm:'Confirming rematch',
      same_game_rotates:'Same game · starting player rotates',
      board_button:'BOARD BUTTON',
      rematch:'REMATCH',
      calibrate_title:'Align projector',
      calibrate_copy:'Move the four corners until the outer ring aligns exactly with the physical dartboard.',
      one_time_setup:'ONE-TIME SETUP',
      reset_center:'Reset round and centered',
      save_calibration:'Save calibration',
      calibration_projector:'Align the outer ring from the Control screen',
      overheat:'OVERHEATED!',
      that_was_close:'THAT WAS CLOSE!',
      charge_lost:'CHARGE LOST',
      export_data:'Export data',
      session_control:'SESSION CONTROL',
      board_error:'BOARD ERROR',
      board_searching:'SEARCHING FOR BOARD',
      controls:'CONTROLS',
      top_left:'Top left',
      top_right:'Top right',
      bottom_right:'Bottom right',
      bottom_left:'Bottom left',
      calibration_note:'Reported projector area: {width} × {height}. “Round and centered” creates an undistorted square with a 5% safety margin on the shorter browser side. Then fine-tune all four corners against the physical dartboard.',
      artwork_theme:'ARTWORK THEME',
      artwork_copy:'Modes without a legacy neon cover automatically use the cartoon artwork.',
      projector_sound:'PROJECTOR SOUND',
      enabled:'Enabled',
      disabled:'Disabled',
      status:'Status',
      status_active:'Active',
      status_running:'Running',
      status_finished:'Finished',
      status_aborted:'Aborted',
      status_interrupted:'Interrupted',
      event_type_game_started:'Game started',
      event_type_game_finished:'Game finished',
      event_type_game_aborted:'Game aborted',
      event_type_throw:'Throw',
      event_type_throw_corrected:'Throw corrected',
      event_type_throw_deleted:'Throw deleted',
      event_type_continue_turn:'Turn continued',
      event_type_next_player:'Player changed',
      event_type_game_action:'Game action',
      event_type_undo:'Correction undone',
      environment_production:'Production game',
      environment_test:'Test game',
      ruleset_version:'Ruleset {version}',
      ring_single:'Single',
      ring_double:'Double',
      ring_triple:'Triple',
      ring_single_bull:'Bull',
      ring_double_bull:'Double Bull',
      ring_miss:'Miss',
      outcome_success:'Hit',
      outcome_partial:'Almost',
      outcome_danger:'Penalty',
      outcome_miss:'Miss',
      outcome_neutral:'Neutral',
      sound_on:'Enable sound',
      sound_off:'Disable sound',
      test_tone:'Test tone',
      autoplay_help:'The projector browser blocked autoplay. Enable autoplay in kiosk mode and reload the projector page.',
      session_setup:'SESSION SETUP',
      team_ready:'TEAM READY',
    },
  };

  const modes = {
    avoid_bomb: ['Avoid the Bomb','Score points – avoid red','Regular hits score, but red bombs subtract points and trigger arcade chaos.'],
    block_drop: ['Block Drop Darts','Build five lines together','Use darts to steer a playful block puzzle. Everyone builds on the same 5×8 board.'],
    boss_fight: ['Boss Fight','Everyone versus the boss','A co-op boss battle with a round limit. Hits deal damage and weak spots deal double damage.'],
    candy_cannon: ['Candy Cannon','Charge, risk, fire','Hits charge your candy cannon. Reach 8–10 charge, then hit a Bull before it overheats.'],
    color_clash: ['Color Clash','Gold scores, red hurts','The dartboard becomes an arcade color field: color determines points, not the classic dart score.'],
    cookie_monster: ['Cookie Monster','Empty the cookie jar','Clear your personal cookie board, avoid mold, and unlock the next batch only when it is empty.'],
    countup: ['Count Up','Every point counts','Score as many points as possible over the selected number of rounds.'],
    cricket: ['Cricket','Close and score','Close 15 through 20 and Bull while scoring on targets your opponents still have open.'],
    dart_sweeper: ['DartSweeper','Clear the minefield together','The 20 numbers become Minesweeper cells. Doubles, Triples, and Bulls reveal extra safe numbers.'],
    darts_bingo: ['Darts Bingo','Complete tasks, make a line','Each player has the same 3×3 card of dart tasks. Win with one line or the full card.'],
    dragon_eggs: ['Dragon Eggs','Collect eggs, avoid dragon fire','Collect golden eggs. Every red scale heats the dragon; the third ignites its fire.'],
    eight_ball: ['8-Ball Darts','Clear your balls','A two-player duel: clear your balls, then hit the Double Bull as the black 8.'],
    ghost_chase: ['Ghost Chase','Catch the jumping ghost','Hit the ghost for a growing three-dart combo. After three misses it escapes to a new target.'],
    heart_chase: ['Heart Chase','Beat the chase score','Beat the previous visit. Failure costs a heart and still sets the new chase score.'],
    king_of_board: ['King of the Board','Capture the dartboard','Every hit captures territory in your color. The largest kingdom wins after the final round.'],
    lightning_round: ['Lightning Round','One task, one dart','Fast mini-challenges: complete the displayed task with your next dart.'],
    mini_golf: ['Mini Golf Darts','Nine holes on the board','Everyone plays the same hole. The earlier you hit the target, the fewer strokes you take.'],
    risk_it: ['Risk It','Bank or risk the Hot Pot','Hits build your pot. Bank after dart 1 or 2—after dart 3, the next player can steal it with one hit.'],
    robin_hood: ['Robin Hood Hunt','Split the Sheriff’s arrows','Chase the previous player’s three targets. Your valid hits become targets for the next player.'],
    simon_says: ['Simon Says','Remember, hit, extend','The projector shows a sequence. Hit its targets in the correct order.'],
    space_defender: ['Space Defender','Stop the waves together','A co-op space adventure: destroy the ships before the invasion reaches ten enemies.'],
    target_rush: ['Target Rush','Hit the glowing target','Hit the exact lit segment for full points; the same number in another ring scores Almost points.'],
    treasure_hunt: ['Treasure Hunt','Find treasure, avoid traps','The board is a treasure map. Hits reveal hidden coins, gold, and traps.'],
    x01: ['X01','Reach exactly zero','The tournament classic: check out precisely from 301, 501, or 701.'],
  };
  const instructionsEn = {
    avoid_bomb:[
      ['Red is dangerous','Red segments are bombs and cost points.'],
      ['Everything else scores','Regular hits score their dart value.'],
      ['Harder each round','After everyone has played, bombs grow evenly or by the new round number.'],
      ['Memory bombs','In Memory mode, half the bombs disappear for two rounds after one visible round, then return.'],
      ['Boom or close call','Bomb hits explode loudly. Directly adjacent segments show “That was close” but score normally.'],
    ],
    block_drop:[
      ['Four large control zones','The four colored arcs move left/right or rotate left/right.'],
      ['Cyan means Drop','Easy uses Doubles, Triples, and Bulls. Medium uses Doubles and Bulls. Hard uses Bulls only.'],
      ['Choose the pace','Classic sinks after each team round. Action sinks after every dart and targets ten lines.'],
      ['Clear lines','Build the selected number of lines together before a block reaches the top.'],
    ],
    boss_fight:[
      ['Deal damage','Every hit reduces the boss’s HP.'],
      ['Weak spots','Golden targets deal double damage.'],
      ['Round limit','Defeat the boss within the selected rounds. Everyone wins; top damage is only an MVP honor.'],
    ],
    candy_cannon:[
      ['Charge to 8, 9, or 10','A Single adds 1, Double 2, Triple 3, and Bull 4 charge.'],
      ['READY? Hit a Bull','When READY appears, fire with a Single or Double Bull: +50 for you, −25 for the leader.'],
      ['11 is too much','Above 10 the cannon overheats and your charge resets to zero.'],
    ],
    color_clash:[
      ['Colors score','Gold +50, cyan +25, green +10, red −25.'],
      ['Classic score is ignored','The color of the hit segment decides the points.'],
      ['Equal chances','Everyone gets the same colors per round—fixed or as the same three-dart sequence.'],
    ],
    cookie_monster:[
      ['Eat the board clean','Hit cookies to remove them for you. A new board appears only after all are gone.'],
      ['Avoid mold','Mold costs points but does not need to be cleared.'],
      ['Bull means milk','Easy gives fixed bonus points. Medium and Hard use milk to double or save your visit.'],
      ['Choose a level','Easy uses large number areas; Medium adds gold; Hard adds colors and Sugar Rush.'],
    ],
    countup:[
      ['Score points','Every hit is added directly to your total.'],
      ['Three darts','A visit consists of three darts.'],
      ['Highest score wins','The highest score after the selected rounds wins.'],
    ],
    cricket:[
      ['Hit the targets','Only 15, 16, 17, 18, 19, 20, and Bull count.'],
      ['Close with three marks','Single counts one mark, Double two, and Triple three.'],
      ['Score while open','Extra marks score while an opponent still has the target open.'],
      ['Win','Close every target and have at least as many points as every opponent.'],
    ],
    dart_sweeper:[
      ['Reveal a number','Singles reveal exactly the hit number.'],
      ['Ring power','A Double reveals one and a Triple two additional safe neighbors.'],
      ['Clear it together','Bull helps scan. Reveal all safe numbers before the final life is lost.'],
    ],
    darts_bingo:[
      ['Fill the card','Every hit can complete one task.'],
      ['Watch the win condition','Win with either the first line or the full card.'],
      ['Equal chances','Everyone plays the same randomly generated task card.'],
    ],
    dragon_eggs:[
      ['Golden egg','A visible egg scores +30 once per round.'],
      ['Red scale','A scale costs 15 points and adds one flame.'],
      ['Dragon fire','The third flame also burns half of your positive points from this visit.'],
    ],
    eight_ball:[
      ['Your balls','Player 1 clears 1–7; Player 2 clears 9–15.'],
      ['A foul ends the visit','A wrong ball, neutral segment, or Miss ends the visit immediately.'],
      ['Black 8','After clearing your balls, win with Double Bull. Hitting it early gives the opponent the win.'],
    ],
    ghost_chase:[
      ['Hit the ghost','Hit the exact marked segment.'],
      ['Chase the combo','Hits within one visit score 40, 50, then 60.'],
      ['The ghost escapes','After three misses it jumps to a new target.'],
      ['Same path','Everyone chases the same ghost sequence each round.'],
    ],
    heart_chase:[
      ['Set the chase','The first player sets a chase score with three darts.'],
      ['Beat it—ties do not count','Failure costs one heart.'],
      ['Last heart wins','Eliminated players are skipped automatically.'],
    ],
    king_of_board:[
      ['Capture territory','Hits capture segments in your color.'],
      ['Easy ring power','A Double takes the whole number. A Triple also takes both neighboring numbers.'],
      ['Steal it back','Hit an opponent’s territory to capture it.'],
      ['Majority wins','The largest board territory after the final round wins.'],
    ],
    lightning_round:[
      ['Read the task','The projector displays the challenge.'],
      ['One dart','Each player gets exactly one dart per task.'],
      ['Success scores','Success scores +25; failure scores 0.'],
    ],
    mini_golf:[
      ['Same hole','Every player throws at the same target.'],
      ['Fewer strokes','A hit with dart 1, 2, or 3 counts that many strokes.'],
      ['Lowest score wins','No hit counts four strokes. Lowest score after the final hole wins.'],
    ],
    risk_it:[
      ['Build the pot','Every hit adds to your unsecured pot.'],
      ['Bank after dart 1 or 2','BANK secures the pot and ends your visit.'],
      ['Dart 3 is the risk','A hit turns its number into the glowing Hot Pot target.'],
      ['The first dart can steal','The next player hits that number to steal the pot. Otherwise it is secured for you.'],
    ],
    robin_hood:[
      ['Chase arrows','Hit the Sheriff targets. Duplicate targets count separately.'],
      ['Split points','A split scores 30 plus the value of the Sheriff’s dart.'],
      ['Pass targets on','Your valid hits become targets for the next player.'],
    ],
    simon_says:[
      ['Remember the sequence','The glowing number groups show the order.'],
      ['Every ring scores','Hit a number in the current group. Single, Double, and Triple are all correct.'],
      ['Bull is a joker','Single Bull and Double Bull always complete the next target.'],
      ['The sequence grows','The shared task grows during the first three rounds.'],
      ['Equal chances','Everyone gets exactly the same sequence in a round.'],
    ],
    space_defender:[
      ['Hit the ships','Hit the exact segment. Singles, Doubles, and Triples deal 1, 2, or 3 damage.'],
      ['Bull laser','Bull hits every active ship at once.'],
      ['Save Earth','After the final wave, clear all remaining ships together.'],
    ],
    target_rush:[
      ['Target lights up','Hit the cyan segment.'],
      ['Almost scores','The right number in the wrong ring scores a few points.'],
      ['Build a combo','Consecutive exact hits earn a bonus.'],
      ['Equal chances','Everyone gets the same sequence of three targets per round.'],
    ],
    treasure_hunt:[
      ['Hidden treasure','Hits reveal hidden contents.'],
      ['Gold pays','Gold and silver score big points.'],
      ['Avoid traps','Red traps cost points.'],
    ],
    x01:[
      ['Count down','Every hit is subtracted from your remaining score.'],
      ['Exactly zero','You must reach exactly zero. With Double Out, the last dart must be a Double.'],
      ['Bust','If you bust, the entire visit resets.'],
    ],
  };

  const exactEn = {
    'Runden':'Rounds','Startbomben':'Starting bombs','Bombenzuwachs':'Bomb growth','Bombensicht':'Bomb visibility',
    'Strafe':'Penalty','Drop-Ziel':'Drop target','Spieltempo':'Pace','Nach Drop':'After drop',
    'Boss HP':'Boss HP','Schwachpunkte':'Weak spots','Rundenlimit':'Round limit',
    'Farbwechsel':'Color change','Spielstufe':'Difficulty','Schwierigkeit':'Difficulty','Zielgröße':'Target size',
    'Sieg':'Win condition','Dracheneier':'Dragon eggs','Geisterpfad':'Ghost path',
    'Herzen':'Hearts','Eroberung':'Capture rules','Ziele':'Targets','Löcher':'Holes',
    'Platz':'Course','Miss':'Miss','Trefferregel':'Hit rule','Wellen':'Waves',
    'Fallen':'Traps','Startpunktzahl':'Starting score','Checkout':'Checkout',
    'Weiterwerfen':'Keep throwing','Zug beenden':'End visit','Nach jeder Runde':'After every round',
    'Nach jedem Dart · gleich für alle':'After every dart · same for everyone',
    'Erste Linie':'First line','Volle Karte':'Full card','Pot verlieren':'Lose the pot',
    'Pot halbieren':'Halve the pot','Leicht · Ganzes Zahlenfeld':'Very easy · whole number',
    'Klassisch · Segment genau':'Classic · exact segment',
    'Leicht · Double-Reihe, Triple-Nachbarn':'Easy · Double row, Triple neighbors',
    'Sehr leicht · Ganzes Zahlenfeld':'Very easy · whole number',
    'Easy · Zahl genügt':'Easy · any ring','Normal · Single/Double exakt':'Normal · exact Single/Double',
    'Schwer · nur Bull':'Hard · Bull only','Mittel · Double oder Bull':'Medium · Double or Bull',
    'Easy · Double, Triple oder Bull':'Easy · Double, Triple, or Bull',
    'Klassisch · 5 Linien':'Classic · 5 lines','Action · 10 Linien, Sink je Dart':'Action · 10 lines, sink each dart',
    'Einfach · Snack Time':'Easy · Snack Time','Mittel · Cookie Hunt':'Medium · Cookie Hunt',
    'Schwer · Sugar Rush':'Hard · Sugar Rush','Gleiches Spiel · Startspieler wechselt':'Same game · starting player rotates',
    'Sehr leicht · 4 Zonen':'Very easy · 4 zones','Leicht · 5 Zonen':'Easy · 5 zones',
    'Mittel · 10 Zonen':'Medium · 10 zones','Schwer · 20 Zahlen':'Hard · 20 numbers',
    'Nach links':'Move left','Links drehen':'Rotate left','Rechts drehen':'Rotate right',
    'Nach rechts':'Move right','Stein droppen':'Drop block','STEUERUNG':'CONTROLS',
    'AKTUELLE AUFGABE':'CURRENT TARGET','SPIELSTATUS':'GAME STATUS',
    'Meide Rot!':'Avoid red!','Gold zählt am meisten!':'Gold scores the most!',
    'Bombe':'Bomb','Versteckt':'Hidden','Immer sichtbar':'Always visible',
    'Memory · zeitweise versteckt':'Memory · temporarily hidden','DAS WAR KNAPP!':'THAT WAS CLOSE!',
    'Der erste Treffer ist garantiert sicher!':'The first hit is guaranteed safe!',
    'ALLE COOKIES ESSEN · SCHIMMEL MEIDEN · BULL = MILCH':'EAT ALL COOKIES · AVOID MOLD · BULL = MILK',
    'MILCH':'MILK','MILCH! +30':'MILK! +30','Schimmel':'Mold',
    'Bull-Milch':'Bull = Milk','Grüner Cookie':'Green Cookie',
    'Keine Krümel':'No crumbs','Hier ist schon alles aufgegessen':'Already eaten',
    'Board erst komplett leer essen':'Clear the whole board first',
    'Turn ×2 / retten':'Visit ×2 / save',
    'Goldene Eier sammeln · rote Schuppen meiden!':'Collect golden eggs · avoid red scales!',
    'GOLDENE EIER SAMMELN · ROTE SCHUPPEN MEIDEN':'COLLECT GOLDEN EGGS · AVOID RED SCALES',
    'GOLDENES EI':'GOLDEN EGG','ROTE SCHUPPE':'RED SCALE','DRACHEN-HITZE':'DRAGON HEAT',
    'Die dritte Schuppe entfacht das Feuer':'The third scale ignites the fire',
    'Noch eine Schuppe: Feuer!':'One more scale: fire!',
    'Gold +50 · Cyan +25 · Grün +10 · Rot -25':'Gold +50 · Cyan +25 · Green +10 · Red -25',
    'BEREIT · JETZT BULL TREFFEN!':'READY · HIT A BULL NOW!',
    'LADUNG AUF 8–10 STELLEN · DANN MIT BULL FEUERN':'CHARGE TO 8–10 · THEN FIRE WITH A BULL',
    'Eröffnet die Jagd!':'Set the chase!','Jagd eröffnen!':'Set the chase!',
    'Räumt eure Kugeln ab!':'Clear your balls!','Erobere die Scheibe!':'Capture the dartboard!',
    'Merke die Sequenz!':'Remember the sequence!','Falsches Feld – Sequenz reset':'Wrong segment – sequence reset',
    'Laser geht vorbei':'Laser misses','LETZTE AUFRÄUMRUNDE!':'FINAL CLEANUP ROUND!',
    'Aufräumrunde!':'Cleanup round!','Bei 10 aktiven Schiffen ist die Erde verloren':'Earth is lost at 10 active ships',
    'Bust – Aufnahme wird zurückgesetzt':'Bust – visit reset',
    'Miss – kein Schaden':'Miss – no damage',
    'OVERHEAT! Ladung verloren':'OVERHEAT! Charge lost',
    'Über 10 überhitzt die Kanone':'Above 10 overheats the cannon',
    'MINENFELD GERÄUMT! Das Team gewinnt!':'MINEFIELD CLEARED! The team wins!',
    'MISS · Das Minenfeld bleibt verdeckt':'MISS · The minefield stays hidden',
    'Single 1 Feld · Double +1 · Triple +2':'Single 1 cell · Double +1 · Triple +2',
    'Für alle liegt dieselbe Bingo-Karte bereit!':'Everyone gets the same Bingo card!',
    'Dieses Ei ist schon leer':'This egg is already empty',
    'Der Geist bleibt':'The ghost stays',
    'WHOOSH! Der Geist ist geflohen':'WHOOSH! The ghost escaped',
    'Fang den Geist!':'Catch the ghost!',
    'EROBERE DIE SCHEIBE!':'CAPTURE THE DARTBOARD!',
    'DOUBLE BOGEY · 4 Schläge':'DOUBLE BOGEY · 4 strokes',
    'Am Loch vorbei':'Missed the hole',
    'Nächstes Loch':'Next hole',
    'Miss – Pot verloren':'Miss – pot lost',
    'UNGESICHERTER POT':'UNSECURED POT','BESITZER':'OWNER',
    'DIEBSTAHL-ZIEL':'HEIST TARGET','TREFFER FÜLLEN DEINEN POT':'HITS BUILD YOUR POT',
    'BANKEN ODER WEITERWERFEN':'BANK OR KEEP THROWING',
    'BANKEN ODER DART 3 RISKIEREN':'BANK OR RISK DART 3',
    'Finaler Hot Pot gesichert':'Final Hot Pot secured',
    'Die Sheriff-Pfeile liegen bereit!':'The Sheriff arrows are ready!',
    'ERDE GERETTET! Das Team gewinnt!':'EARTH SAVED! The team wins!',
    'Die Flotte entkommt · Team-Niederlage':'The fleet escapes · team defeat',
    'INVASION! Zehn Schiffe haben die Erde erreicht':'INVASION! Ten ships reached Earth',
    'Miss – Combo reset':'Miss – combo reset','Miss – kein Fund':'Miss – nothing found',
    'Treffer decken Schätze auf!':'Hits reveal treasure!',
    'Triff ein Double':'Hit any Double','Triff ein Triple':'Hit any Triple',
    'Triff eine Zahl über 15':'Hit a number above 15','Triff eine Zahl unter 10':'Hit a number below 10',
    'Triff Bull':'Hit Bull','Triff eine gerade Zahl':'Hit an even number',
    'Offene Cricket-Ziele':'Open Cricket targets','NOCH ZU SCHLIESSEN':'STILL TO CLOSE',
    'MINEN':'MINES','GEFUNDEN':'FOUND','Schwarze 8':'Black 8',
    'Keine Bingo-Aufgabe getroffen':'No Bingo task completed','Miss – kein Bingo':'Miss – no Bingo',
    'Kein Sheriff-Pfeil gespalten':'No Sheriff arrow split',
    'Freie Runde – lege neue Pfeile!':'Free round – set new arrows!',
    'Keine Sessionpunkte in diesem Spiel.':'No session points in this game.',
    'Das Team gewinnt gemeinsam.':'The team wins together.',
    '+1 pro Runde':'+1 per round','+ Rundennummer':'+ round number',
    'Explorer · 3 Minen / 5 Leben':'Explorer · 3 mines / 5 lives',
    'Classic · 5 Minen / 3 Leben':'Classic · 5 mines / 3 lives',
    'Expert · 7 Minen / 2 Leben':'Expert · 7 mines / 2 lives',
    'Normal · Alle Ringe':'Normal · all rings',
    'Komet':'Comet','Stern':'Star','Blitz':'Bolt','Schlange':'Snake',
    'Planet':'Planet','Krone':'Crown','Ziel':'Target','Feuer':'Fire',
    'Einhorn':'Unicorn','Ninja':'Ninja','Roboter':'Robot','Party':'Party',
  };

  function format(template, vars){
    return String(template).replace(/\{(\w+)\}/g, (_,key)=>vars?.[key] ?? '');
  }
  function t(key, lang, vars){
    const locale=ui[lang] || ui.de;
    return format(locale[key] ?? ui.de[key] ?? key, vars);
  }
  function text(value, lang){
    const raw=String(value ?? '');
    if(lang!=='en') return raw;
    if(exactEn[raw]) return exactEn[raw];
    const runtimeEn=raw
      .replace(/^(.+) gewinnt die Keksdose!$/, '$1 wins the cookie jar!')
      .replace(/^(.+) gewinnt die Candy Cannon!$/, '$1 wins Candy Cannon!')
      .replace(/^(.+) gewinnt den Color Clash!$/, '$1 wins Color Clash!')
      .replace(/^(.+) gewinnt Count Up!$/, '$1 wins Count Up!')
      .replace(/^(.+) gewinnt den Target Rush!$/, '$1 wins Target Rush!')
      .replace(/^(.+) gewinnt Lightning!$/, '$1 wins Lightning!')
      .replace(/^(.+) gewinnt Simon Says!$/, '$1 wins Simon Says!')
      .replace(/^(.+) gewinnt Risk It!$/, '$1 wins Risk It!')
      .replace(/^(.+) überlebt Avoid the Bomb!$/, '$1 survives Avoid the Bomb!')
      .replace(/^(.+) hütet den Drachenschatz!$/, '$1 guards the dragon treasure!')
      .replace(/^(.+) ist der beste Geisterjäger!$/, '$1 is the best ghost hunter!')
      .replace(/^(.+) regiert die Scheibe!$/, '$1 rules the dartboard!')
      .replace(/^(.+) findet den größten Schatz!$/, '$1 finds the greatest treasure!')
      .replace(/^(.+) ist der beste Pfeilspalter!$/, '$1 is the best arrow splitter!')
      .replace(/^(.+) gewinnt die Herzjagd!$/, '$1 wins Heart Chase!')
      .replace(/^MILK! Turn gerettet ([+-][0-9]+)$/, 'MILK! Visit saved $1')
      .replace(/^MILK! Turn verdoppelt ([+-][0-9]+)$/, 'MILK! Visit doubled $1')
      .replace(/^SCHIMMEL! (-[0-9]+)$/, 'MOLD! $1')
      .replace(/^BOARD GEPUTZT! (\+[0-9]+) · Neue Cookies!$/, 'BOARD CLEARED! $1 · New cookies!')
      .replace(/^SUGAR RUSH BEREIT · nächster Cookie doppelt$/, 'SUGAR RUSH READY · next cookie scores double')
      .replace(/^Serie ([0-9]+)\/3 · Board erst komplett leer essen$/, 'Streak $1/3 · clear the whole board first')
      .replace(/^(.+) hat BINGO · Ausgleichsrunde läuft$/, '$1 has BINGO · equalizer round in progress')
      .replace(/^Schuppe! (-[0-9]+) · Hitze ([0-9]+)\/3$/, 'Scale! $1 · Heat $2/3')
      .replace(/^(.+): kein Cricket-Ziel$/, '$1: not a Cricket target')
      .replace(/^Boss gewinnt mit ([0-9]+) HP!$/, 'Boss wins with $1 HP!')
      .replace(/^(.+) macht ([0-9]+) Schaden$/, '$1 deals $2 damage')
      .replace(/^FLÄCHENLASER! ([0-9]+) Schaden an allen$/, 'AREA LASER! $1 damage to all')
      .replace(/^(.+) getroffen · ([0-9]+) Schaden$/, '$1 hit · $2 damage')
      .replace(/^Welle ([0-9]+) ist gelandet!$/, 'Wave $1 has landed!')
      .replace(/^Runde ([0-9]+): Eine neue Bombe ist aktiv!$/, 'Round $1: one new bomb is active!')
      .replace(/^Runde ([0-9]+): ([0-9]+) neue Bomben sind aktiv!$/, 'Round $1: $2 new bombs are active!')
      .replace(/^Runde ([0-9]+): Stein fällt eine Zeile$/, 'Round $1: block drops one row')
      .replace(/^([0-9]+) sichere Felder \+ ([0-9]+)$/, '$1 safe cells + $2')
      .replace(/^BOOM auf (.+)$/, 'BOOM on $1')
      .replace(/^(.+) ist bereits bekannt$/, '$1 is already known')
      .replace(/^(.+) ist bereits aufgedeckt$/, '$1 is already revealed')
      .replace(/^Loch ([0-9]+):/, 'Hole $1:')
      .replace(/^(.+) gewinnt den Platz mit ([0-9]+) Schlägen!$/, '$1 wins the course with $2 strokes!')
      .replace(/^(.+) überspringt · 4 Schläge$/, '$1 skips · 4 strokes')
      .replace(/^Miss – Pot halbiert auf ([0-9]+)$/, 'Miss – pot halved to $1')
      .replace(/^(.+) – banken oder riskieren\?$/, '$1 – bank or risk it?')
      .replace(/^(.+) – BANK oder RISK\?$/, '$1 – BANK or RISK?')
      .replace(/^Jagd eröffnet: ([0-9]+)$/, 'Chase set: $1')
      .replace(/^(.+) muss strikt mehr werfen$/, '$1 must score strictly more')
      .replace(/^Falsches Feld: (.+)$/, 'Wrong segment: $1')
      .replace(/^Weiter: Z([0-9]+)$/, 'Next: Z$1')
      .replace(/^(.+): leer$/, '$1: empty')
      .replace(/^(.+) überspringt · Pot verloren$/, '$1 skips · pot lost')
      .replace(/^8-Ball zu früh! (.+) gewinnt$/, 'Black 8 too early! $1 wins')
      .replace(/^Kugel ([0-9]+) versenkt! (\+[0-9]+)$/, 'Ball $1 cleared! $2')
      .replace(/^([0-9]+) PUNKTE$/, '$1 POINTS')
      .replace(/^TRIFF (.+) MIT DART 1 · STIEHL ([0-9]+)$/, 'HIT $1 WITH DART 1 · STEAL $2')
      .replace(/^HOT POT ([0-9]+) · ZIEL (.+)$/, 'HOT POT $1 · TARGET $2')
      .replace(/^(.+): Triff (.+) mit Dart 1$/, '$1: Hit $2 with dart 1')
      .replace(/^(.+) kann mit Dart 1 auf (.+) stehlen$/, '$1 can steal with dart 1 on $2')
      .replace(/^Pot ([0-9]+) · BANK oder weiter\?$/, 'Pot $1 · BANK or keep going?')
      .replace(/^HEIST! (.+) stiehlt ([0-9]+) von (.+)$/, 'HEIST! $1 steals $2 from $3')
      .replace(/^SAFE! (.+) bankt ([0-9]+)$/, 'SAFE! $1 banks $2')
      .replace(/^Miss · halber Pot gesichert \+([0-9]+)$/, 'Miss · half the pot secured +$1')
      .replace(/^Miss · Pot halbiert auf ([0-9]+)$/, 'Miss · pot halved to $1')
      .replace(/^Miss · eigener Pot verloren$/, 'Miss · own pot lost')
      .replace(/^(.+) bankt \+([0-9]+)$/, '$1 banks +$2')
      .replace(/^(.+) überspringt · Pot ([0-9]+) verloren$/, '$1 skips · pot $2 lost');
    if(runtimeEn!==raw) return runtimeEn;
    return raw
      .replace(/^([0-9]+) Runden$/, '$1 rounds')
      .replace(/^([0-9]+) Bomben$/, '$1 bombs')
      .replace(/^([0-9]+) Eier$/, '$1 eggs')
      .replace(/^([0-9]+) Herzen$/, '$1 hearts')
      .replace(/^([0-9]+) Wellen$/, '$1 waves')
      .replace(/^([0-9]+) Löcher$/, '$1 holes')
      .replace(/^([0-9]+) Fallen$/, '$1 traps')
      .replace(/^Runde ([0-9]+):/, 'Round $1:')
      .replace(/^Welle ([0-9]+)/, 'Wave $1')
      .replace(/^Loch ([0-9]+)/, 'Hole $1')
      .replace(/^Triff (.+)!$/, 'Hit $1!')
      .replace(/^Fang (.+)!$/, 'Catch $1!')
      .replace(/^Schlag ([0-9]+)!$/, 'Beat $1!')
      .replace(/^([A-Za-zÀ-ž0-9 _-]+) gewinnt!$/, '$1 wins!')
      .replace(/Eine neue Bombe ist aktiv/g, 'One new bomb is active')
      .replace(/([0-9]+) neue Bomben sind aktiv/g, '$1 new bombs are active')
      .replace(/Die versteckten Bomben sind wieder sichtbar/g, 'The hidden bombs are visible again')
      .replace(/([0-9]+) Bomben sind für zwei Runden versteckt/g, '$1 bombs are hidden for two rounds')
      .replace(/([0-9]+) sichtbar/g, '$1 visible')
      .replace(/([0-9]+) versteckt/g, '$1 hidden')
      .replace(/meide alle Bomben/gi, 'avoid every bomb')
      .replace(/([0-9]+) Bomben/g, '$1 bombs')
      .replace(/([0-9]+) sichere Felder übrig/g, '$1 safe fields left')
      .replace(/([0-9]+) Cookies übrig/g, '$1 cookies left')
      .replace(/([0-9]+) Kugeln übrig/g, '$1 balls left')
      .replace(/([0-9]+) Schiffe/g, '$1 ships')
      .replace(/([0-9]+) Linien/g, '$1 lines')
      .replace(/([0-9]+) Schläge(?:n)?/g, '$1 strokes')
      .replace(/Ladung/g, 'Charge')
      .replace(/FLAMMEN/g, 'FLAMES')
      .replace(/Schaden/g, 'damage')
      .replace(/meide Rot/gi, 'avoid red')
      .replace(/Alle bauen gemeinsam/g, 'Everyone builds together')
      .replace(/noch offen/g, 'remaining')
      .replace(/Aktuelle Jagd/g, 'Current chase')
      .replace(/muss strikt mehr werfen/g, 'must score strictly more')
      .replace(/Drei Darts legen die erste Jagd fest/g, 'Three darts set the first chase')
      .replace(/Neue Jagd/g, 'New chase')
      .replace(/Schuppe/g, 'Scale')
      .replace(/Hitze/g, 'Heat')
      .replace(/bereits gefunden/g, 'already found')
      .replace(/leer/g, 'empty');
  }
  function mode(mode, lang){
    if(lang!=='en' || !mode) return mode;
    const copy=modes[mode.slug];
    if(!copy) return mode;
    const instructions=(instructionsEn[mode.slug] || []).map((item,index)=>({
      ...(mode.instructions?.[index] || {}),
      title:item[0],
      body:item[1],
    }));
    return {
      ...mode,
      title:copy[0],
      tagline:copy[1],
      description:copy[2],
      instructions:instructions.length ? instructions : mode.instructions,
    };
  }

  window.SDB_I18N={t,text,mode};
})();
