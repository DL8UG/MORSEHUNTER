//! The window in the languages besides English and German. Each line is the English text
//! as it stands at `tr(...)` in the code, then French, Spanish, Portuguese, Italian,
//! Polish and Russian. A test checks that every text in the code has its line here.

use std::collections::HashMap;
use std::sync::OnceLock;

use mr_sim::settings::Lang;

type Line = (&'static str, [&'static str; 6]);

#[rustfmt::skip]
const TABLE: &[Line] = &[
    // pages
    ("Operate", ["Trafic", "Operar", "Operar", "Operativo", "Praca", "Работа"]),
    ("Options", ["Options", "Opciones", "Opções", "Opzioni", "Opcje", "Настройки"]),
    ("Help", ["Aide", "Ayuda", "Ajuda", "Aiuto", "Pomoc", "Справка"]),
    ("A run is on, the keys still work it.", [
        "Une session est en cours, les touches la pilotent toujours.",
        "Hay una sesión en curso, las teclas siguen manejándola.",
        "Há uma sessão em curso, as teclas continuam a controlá-la.",
        "Una sessione è in corso, i tasti continuano a gestirla.",
        "Trwa sesja, klawisze nadal nią sterują.",
        "Идёт сеанс, клавиши по-прежнему им управляют.",
    ]),

    // header
    ("Stopped", ["Arrêté", "Detenido", "Parado", "Fermo", "Zatrzymano", "Остановлено"]),
    ("Single calls", ["Appels isolés", "Llamadas sueltas", "Chamadas isoladas", "Chiamate singole", "Pojedyncze wywołania", "Одиночные вызовы"]),
    ("WPX competition", ["Concours WPX", "Concurso WPX", "Concurso WPX", "Gara WPX", "Zawody WPX", "Соревнование WPX"]),
    ("HST competition", ["Concours HST", "Concurso HST", "Concurso HST", "Gara HST", "Zawody HST", "Соревнование HST"]),
    ("QSOs per hour", ["QSO par heure", "QSOs por hora", "QSOs por hora", "QSO all'ora", "QSO na godzinę", "QSO в час"]),
    ("Calling", ["Appelants", "Llamando", "Chamando", "Chiamanti", "Wołający", "Зовут"]),
    (", fixed by code", [", fixé par code", ", fijado por código", ", fixado por código", ", fissato dal codice", ", ustalone kodem", ", задано кодом"]),
    ("F9 or Alt-R starts a pile-up", [
        "F9 ou Alt-R lance un pile-up",
        "F9 o Alt-R inicia un pile-up",
        "F9 ou Alt-R inicia um pile-up",
        "F9 o Alt-R avvia un pile-up",
        "F9 lub Alt-R uruchamia pile-up",
        "F9 или Alt-R запускает пайл-ап",
    ]),
    ("No score", ["Pas de score", "Sin puntuación", "Sem pontuação", "Nessun punteggio", "Bez wyniku", "Без зачёта"]),
    ("debug is on", ["le debug est actif", "el debug está activo", "o debug está ativo", "il debug è attivo", "debug jest włączony", "отладка включена"]),
    ("debug was on", ["le debug était actif", "el debug estuvo activo", "o debug esteve ativo", "il debug era attivo", "debug był włączony", "отладка была включена"]),
    ("verified score", ["score vérifié", "puntuación verificada", "pontuação verificada", "punteggio verificato", "wynik sprawdzony", "проверенный счёт"]),

    // entry and keys
    ("Call", ["Indicatif", "Indicativo", "Indicativo", "Nominativo", "Znak", "Позывной"]),
    ("Keyed text shows here", [
        "Le texte manipulé s'affiche ici",
        "Aquí aparece el texto manipulado",
        "O texto manipulado aparece aqui",
        "Qui appare il testo manipolato",
        "Tu pojawia się nadany tekst",
        "Здесь появляется переданный текст",
    ]),
    ("Your number", ["Votre numéro", "Tu número", "Seu número", "Il tuo numero", "Twój numer", "Ваш номер"]),
    ("Exchange", ["Report", "Reporte", "Reporte", "Rapporto", "Raport", "Обмен"]),
    ("My call", ["Mon call", "Mi call", "Meu call", "Mio call", "Mój znak", "Мой позыв"]),
    ("His call", ["Son call", "Su call", "Seu call", "Suo call", "Jego znak", "Его позыв"]),

    // log area
    ("Log", ["Log", "Log", "Log", "Log", "Log", "Лог"]),
    ("High scores", ["Classement", "Clasificación", "Classificação", "Classifica", "Ranking", "Рейтинг"]),
    ("Sent", ["Envoyé", "Enviado", "Enviado", "Inviato", "Wysłane", "Передано"]),
    ("Skill", ["Performance", "Rendimiento", "Desempenho", "Rendimento", "Sprawność", "Мастерство"]),
    ("From a code", ["Par code", "Por código", "Por código", "Da codice", "Z kodu", "По коду"]),
    ("Debug is on: runs score nothing and stay out of this list.", [
        "Le debug est actif : les sessions ne marquent aucun point et n'entrent pas dans ce classement.",
        "El debug está activo: las sesiones no puntúan y no entran en esta clasificación.",
        "O debug está ativo: as sessões não pontuam e ficam fora desta classificação.",
        "Il debug è attivo: le sessioni non fanno punti e restano fuori da questa classifica.",
        "Debug jest włączony: sesje nie dostają punktów i nie trafiają do tego rankingu.",
        "Отладка включена: сеансы не получают очков и не попадают в этот рейтинг.",
    ]),
    ("Every run that ends by itself lands here, ranked by skill points per hour.", [
        "Chaque session qui se termine d'elle-même apparaît ici, classée par points de performance par heure.",
        "Cada sesión que termina por sí sola aparece aquí, ordenada por puntos de rendimiento por hora.",
        "Cada sessão que termina sozinha aparece aqui, ordenada por pontos de desempenho por hora.",
        "Ogni sessione che finisce da sola compare qui, ordinata per punti di rendimento all'ora.",
        "Każda sesja, która kończy się sama, trafia tutaj, według punktów sprawności na godzinę.",
        "Каждый сеанс, завершившийся сам, попадает сюда, по очкам мастерства в час.",
    ]),
    ("A pile-up or single calls run started from a code and played to the end lands here.", [
        "Une session pile-up ou appels isolés lancée par un code et jouée jusqu'au bout apparaît ici.",
        "Aquí aparece una sesión de pile-up o llamadas sueltas iniciada con un código y jugada hasta el final.",
        "Aqui aparece uma sessão de pile-up ou chamadas isoladas iniciada por um código e jogada até o fim.",
        "Qui compare una sessione pile-up o chiamate singole avviata da un codice e giocata fino alla fine.",
        "Tu trafia sesja pile-up lub pojedynczych wywołań uruchomiona z kodu i rozegrana do końca.",
        "Сюда попадает пайл-ап или одиночные вызовы, запущенные по коду и сыгранные до конца.",
    ]),
    ("A finished competition lands here. Stopping it early gives no score.", [
        "Un concours terminé apparaît ici. L'arrêter avant la fin ne donne aucun score.",
        "Aquí aparece un concurso terminado. Detenerlo antes no da puntuación.",
        "Aqui aparece um concurso terminado. Pará-lo antes não dá pontuação.",
        "Qui compare una gara finita. Fermarla prima non dà punteggio.",
        "Tu trafiają ukończone zawody. Przerwanie wcześniej nie daje wyniku.",
        "Сюда попадает законченное соревнование. Остановка раньше не даёт зачёта.",
    ]),
    ("Pts/h are skill points per hour. Per good QSO 10 points at 25 wpm, in proportion to the speed. Each question -20 %, a busted QSO takes off half its worth, each band effect +10 %, a head start +10 % from 1 h, +20 % from 6 h.", [
        "Pts/h sont des points de performance par heure. Par QSO correct 10 points à 25 wpm, proportionnellement à la vitesse. Chaque question -20 %, un QSO erroné retire la moitié de sa valeur, chaque effet de bande +10 %, une avance de 1 h +10 %, de 6 h +20 %.",
        "Pts/h son puntos de rendimiento por hora. Por QSO correcto 10 puntos a 25 wpm, en proporción a la velocidad. Cada pregunta -20 %, un QSO erróneo resta la mitad de su valor, cada efecto de banda +10 %, una ventaja de 1 h +10 %, de 6 h +20 %.",
        "Pts/h são pontos de desempenho por hora. Por QSO correto 10 pontos a 25 wpm, em proporção à velocidade. Cada pergunta -20 %, um QSO errado desconta metade do seu valor, cada efeito de banda +10 %, uma vantagem de 1 h +10 %, de 6 h +20 %.",
        "Pt/h sono punti di rendimento all'ora. Per QSO buono 10 punti a 25 wpm, in proporzione alla velocità. Ogni domanda -20 %, un QSO errato sottrae metà del suo valore, ogni effetto di banda +10 %, un anticipo di 1 h +10 %, di 6 h +20 %.",
        "Pkt/h to punkty sprawności na godzinę. Za dobre QSO 10 punktów przy 25 wpm, proporcjonalnie do tempa. Każde pytanie -20 %, błędne QSO odejmuje połowę swojej wartości, każdy efekt pasma +10 %, zawody trwające od 1 h +10 %, od 6 h +20 %.",
        "Оч/ч это очки мастерства в час. За хорошее QSO 10 очков при 25 wpm, пропорционально скорости. Каждый переспрос -20 %, ошибочное QSO отнимает половину своей цены, каждый эффект диапазона +10 %, контест идёт уже 1 ч +10 %, 6 ч +20 %.",
    ]),
    ("Pl.", ["Pl.", "Pos", "Pos", "Pos", "Mc.", "М."]),
    ("Pts/h", ["Pts/h", "Pts/h", "Pts/h", "Pt/h", "Pkt/h", "Оч/ч"]),
    ("Sum", ["Somme", "Suma", "Soma", "Somma", "Suma", "Сумма"]),
    ("Ask", ["?", "?", "?", "?", "?", "?"]),
    ("Mode", ["Mode", "Modo", "Modo", "Modo", "Tryb", "Режим"]),
    ("Date", ["Date", "Fecha", "Data", "Data", "Data", "Дата"]),
    ("Pref", ["Préf", "Pref", "Pref", "Pref", "Pref", "Преф"]),
    ("Score", ["Points", "Puntos", "Pontos", "Punti", "Wynik", "Очки"]),
    ("new", ["nouveau", "nuevo", "novo", "nuovo", "nowy", "новый"]),
    ("Click to copy the code", [
        "Clic pour copier le code",
        "Clic para copiar el código",
        "Clique para copiar o código",
        "Clic per copiare il codice",
        "Kliknij, aby skopiować kod",
        "Щелчок копирует код",
    ]),
    ("Time", ["Heure", "Hora", "Hora", "Ora", "Czas", "Время"]),
    ("Received", ["Reçu", "Recibido", "Recebido", "Ricevuto", "Odebrane", "Принято"]),
    ("Country", ["Pays", "País", "País", "Paese", "Kraj", "Страна"]),
    (" ?", [" ?", " ?", " ?", " ?", " ?", " ?"]),
    ("Skl", ["Perf", "Rend", "Des", "Rend", "Spr", "Маст"]),
    ("Press F9 or Alt-R for a pile-up. Enter calls CQ, then the Enter key works the whole contact.", [
        "F9 ou Alt-R lance un pile-up. Entrée appelle CQ, puis la touche Entrée mène tout le QSO.",
        "F9 o Alt-R inicia un pile-up. Intro llama CQ y luego la tecla Intro lleva todo el QSO.",
        "F9 ou Alt-R inicia um pile-up. Enter chama CQ e depois a tecla Enter conduz todo o QSO.",
        "F9 o Alt-R avvia un pile-up. Invio chiama CQ, poi il tasto Invio porta avanti tutto il QSO.",
        "F9 lub Alt-R uruchamia pile-up. Enter woła CQ, potem klawisz Enter prowadzi całe QSO.",
        "F9 или Alt-R запускает пайл-ап. Enter даёт CQ, дальше клавиша Enter ведёт всё QSO.",
    ]),
    ("Type the call you heard and press Enter.", [
        "Tapez l'indicatif entendu et appuyez sur Entrée.",
        "Escribe el indicativo que oíste y pulsa Intro.",
        "Digite o indicativo que ouviu e pressione Enter.",
        "Scrivi il nominativo che hai sentito e premi Invio.",
        "Wpisz usłyszany znak i naciśnij Enter.",
        "Введите услышанный позывной и нажмите Enter.",
    ]),

    // side column
    ("Stop  F10", ["Arrêt  F10", "Parar  F10", "Parar  F10", "Stop  F10", "Stop  F10", "Стоп  F10"]),
    ("Single", ["Isolés", "Sueltas", "Isoladas", "Singole", "Pojedyncze", "Одиночные"]),
    ("No sound", ["Pas de son", "Sin sonido", "Sem som", "Nessun suono", "Brak dźwięku", "Нет звука"]),
    ("Code of this run, conditions fixed. Click to copy.", [
        "Code de cette session, conditions fixées. Clic pour copier.",
        "Código de esta sesión, condiciones fijas. Clic para copiar.",
        "Código desta sessão, condições fixas. Clique para copiar.",
        "Codice di questa sessione, condizioni fisse. Clic per copiare.",
        "Kod tej sesji, warunki ustalone. Kliknij, aby skopiować.",
        "Код этого сеанса, условия заданы. Щелчок копирует.",
    ]),
    ("Code of this run. Click to copy.", [
        "Code de cette session. Clic pour copier.",
        "Código de esta sesión. Clic para copiar.",
        "Código desta sessão. Clique para copiar.",
        "Codice di questa sessione. Clic per copiare.",
        "Kod tej sesji. Kliknij, aby skopiować.",
        "Код этого сеанса. Щелчок копирует.",
    ]),
    ("Raw", ["Brut", "Bruto", "Bruto", "Grezzo", "Surowy", "Сырой"]),
    ("Verified", ["Vérifié", "Verificado", "Verificado", "Verificato", "Sprawdzony", "Проверен"]),
    ("Points", ["Points", "Puntos", "Pontos", "Punti", "Punkty", "Очки"]),
    ("Prefixes", ["Préfixes", "Prefijos", "Prefixos", "Prefissi", "Prefiksy", "Префиксы"]),
    ("QSOs per 5 minutes", ["QSO par 5 minutes", "QSOs cada 5 minutos", "QSOs a cada 5 minutos", "QSO ogni 5 minuti", "QSO na 5 minut", "QSO за 5 минут"]),

    // result strip
    ("The contest is over", ["Le concours est terminé", "El concurso ha terminado", "O concurso terminou", "La gara è finita", "Zawody zakończone", "Соревнование окончено"]),
    ("The run is over", ["La session est terminée", "La sesión ha terminado", "A sessão terminou", "La sessione è finita", "Sesja zakończona", "Сеанс окончен"]),
    ("prefixes", ["préfixes", "prefijos", "prefixos", "prefissi", "prefiksów", "префиксов"]),
    ("No score: debug was on during the run. Not in the high scores.", [
        "Pas de score : le debug était actif pendant la session. Pas d'entrée au classement.",
        "Sin puntuación: el debug estuvo activo durante la sesión. No entra en la clasificación.",
        "Sem pontuação: o debug esteve ativo durante a sessão. Não entra na classificação.",
        "Nessun punteggio: il debug era attivo durante la sessione. Non entra in classifica.",
        "Bez wyniku: debug był włączony w trakcie sesji. Brak wpisu w rankingu.",
        "Без зачёта: во время сеанса была включена отладка. В рейтинг не попадает.",
    ]),
    (", new record", [", nouveau record", ", nuevo récord", ", novo recorde", ", nuovo record", ", nowy rekord", ", новый рекорд"]),
    ("per hour", ["par heure", "por hora", "por hora", "all'ora", "na godzinę", "в час"]),
    ("in total", ["au total", "en total", "no total", "in totale", "łącznie", "всего"]),
    ("questions", ["questions", "preguntas", "perguntas", "domande", "pytań", "переспросов"]),
    ("busted", ["erronés", "erróneos", "errados", "errati", "błędnych", "ошибочных"]),
    ("Close", ["Fermer", "Cerrar", "Fechar", "Chiudi", "Zamknij", "Закрыть"]),
    ("Same run for someone else:", [
        "Même session pour quelqu'un d'autre :",
        "La misma sesión para otra persona:",
        "A mesma sessão para outra pessoa:",
        "La stessa sessione per qualcun altro:",
        "Ta sama sesja dla kogoś innego:",
        "Тот же сеанс для другого:",
    ]),
    ("Copy", ["Copier", "Copiar", "Copiar", "Copia", "Kopiuj", "Копировать"]),

    // sent tab
    ("by hand", ["à la main", "a mano", "à mão", "a mano", "ręcznie", "вручную"]),
    ("stopped", ["interrompu", "interrumpido", "interrompido", "interrotto", "przerwane", "прервано"]),
    ("Start a run. Everything that is sent appears here.", [
        "Lancez une session. Tout ce qui est émis apparaît ici.",
        "Inicia una sesión. Todo lo que se transmite aparece aquí.",
        "Inicie uma sessão. Tudo o que é transmitido aparece aqui.",
        "Avvia una sessione. Tutto ciò che viene trasmesso appare qui.",
        "Uruchom sesję. Wszystko, co jest nadawane, pojawia się tutaj.",
        "Запустите сеанс. Всё, что передаётся, появится здесь.",
    ]),

    // options
    ("North America", ["Amérique du Nord", "Norteamérica", "América do Norte", "Nord America", "Ameryka Północna", "Северная Америка"]),
    ("South America", ["Amérique du Sud", "Sudamérica", "América do Sul", "Sud America", "Ameryka Południowa", "Южная Америка"]),
    ("Europe", ["Europe", "Europa", "Europa", "Europa", "Europa", "Европа"]),
    ("Asia", ["Asie", "Asia", "Ásia", "Asia", "Azja", "Азия"]),
    ("Oceania", ["Océanie", "Oceanía", "Oceania", "Oceania", "Oceania", "Океания"]),
    ("Africa", ["Afrique", "África", "África", "Africa", "Afryka", "Африка"]),
    ("Straight key", ["Pioche", "Manipulador vertical", "Manipulador vertical", "Tasto verticale", "Klucz sztorcowy", "Простой ключ"]),
    ("locked during a run", ["verrouillé pendant la session", "bloqueado durante la sesión", "bloqueado durante a sessão", "bloccato durante la sessione", "zablokowane w trakcie sesji", "заблокировано во время сеанса"]),
    ("fixed by code", ["fixé par code", "fijado por código", "fixado por código", "fissato dal codice", "ustalone kodem", "задано кодом"]),
    ("Station", ["Station", "Estación", "Estação", "Stazione", "Stacja", "Станция"]),
    ("Operator", ["Opérateur", "Operador", "Operador", "Operatore", "Operator", "Оператор"]),
    ("Speed and sound", ["Vitesse et son", "Velocidad y sonido", "Velocidade e som", "Velocità e suono", "Tempo i dźwięk", "Скорость и звук"]),
    ("Speed", ["Vitesse", "Velocidad", "Velocidade", "Velocità", "Tempo", "Скорость"]),
    ("Pitch", ["Tonalité", "Tono", "Tom", "Tono", "Wysokość tonu", "Тон"]),
    ("Callers from", ["Appelants de", "Llamadas desde", "Chamadas de", "Chiamanti da", "Wołający od", "Зовущие от"]),
    ("Callers up to", ["Appelants jusqu'à", "Llamadas hasta", "Chamadas até", "Chiamanti fino a", "Wołający do", "Зовущие до"]),
    ("Callers who answer from now on, those already calling keep their speed.", [
        "Pour les stations qui répondent désormais, celles qui appellent déjà gardent leur vitesse.",
        "Para las estaciones que respondan a partir de ahora, las que ya llaman mantienen su velocidad.",
        "Para as estações que responderem daqui em diante, as que já chamam mantêm a velocidade.",
        "Per le stazioni che rispondono da ora in poi, quelle che già chiamano tengono la loro velocità.",
        "Dla stacji, które odpowiedzą od teraz, te, które już wołają, zachowują swoje tempo.",
        "Для станций, которые ответят теперь, уже зовущие сохраняют свою скорость.",
    ]),
    ("Bandwidth", ["Bande passante", "Ancho de banda", "Largura de banda", "Larghezza di banda", "Szerokość pasma", "Полоса"]),
    ("Sidetone", ["Écoute locale", "Tono local", "Tom local", "Tono locale", "Podsłuch", "Самоконтроль"]),
    ("QSK: hear the band between elements", [
        "QSK : entendre la bande entre les éléments",
        "QSK: oír la banda entre los elementos",
        "QSK: ouvir a banda entre os elementos",
        "QSK: sentire la banda tra gli elementi",
        "QSK: słychać pasmo między elementami",
        "QSK: слышать эфир между посылками",
    ]),
    ("set by the competition", ["imposé par le concours", "fijado por el concurso", "definido pelo concurso", "imposto dalla gara", "ustalone przez zawody", "задано соревнованием"]),
    ("Band conditions", ["Conditions de bande", "Condiciones de banda", "Condições de banda", "Condizioni di banda", "Warunki na paśmie", "Условия на диапазоне"]),
    ("Activity", ["Activité", "Actividad", "Atividade", "Attività", "Aktywność", "Активность"]),
    ("Contest running for", ["Concours en cours depuis", "Concurso en marcha desde hace", "Concurso rodando há", "Gara in corso da", "Zawody trwają od", "Контест идёт уже"]),
    ("The callers' serial numbers start as high as after this many hours. At 0, as in the original, everybody starts with you.", [
        "Les numéros des stations qui appellent sont aussi hauts qu'après autant d'heures. À 0, comme dans l'original, tout le monde commence avec vous.",
        "Los números de las estaciones que llaman van tan altos como tras tantas horas. Con 0, como en el original, todos empiezan contigo.",
        "Os números das estações que chamam ficam tão altos quanto depois de tantas horas. Com 0, como no original, todos começam com você.",
        "I numeri delle stazioni che chiamano sono alti come dopo altrettante ore. A 0, come nell'originale, tutti iniziano con te.",
        "Numery wołających stacji są tak wysokie jak po tylu godzinach. Przy 0, jak w oryginale, wszyscy zaczynają razem z tobą.",
        "Номера зовущих станций такие, как после стольких часов. При 0, как в оригинале, все начинают вместе с вами.",
    ]),
    ("Run length", ["Durée de session", "Duración de la sesión", "Duração da sessão", "Durata della sessione", "Długość sesji", "Длительность сеанса"]),
    ("Practice", ["Entraînement", "Práctica", "Treino", "Allenamento", "Trening", "Тренировка"]),
    ("Competition", ["Concours", "Concurso", "Concurso", "Gara", "Zawody", "Соревнование"]),
    ("Where callers come from", ["Origine des appelants", "De dónde llaman", "De onde vêm as chamadas", "Da dove chiamano", "Skąd wołają", "Откуда зовут"]),
    ("Key by hand", ["Manipuler à la main", "Manipular a mano", "Manipular à mão", "Manipolare a mano", "Nadawanie ręczne", "Передача вручную"]),
    ("A keyed TU logs the contact", [
        "Un TU manipulé enregistre le QSO",
        "Un TU manipulado registra el QSO",
        "Um TU manipulado registra o QSO",
        "Un TU manipolato registra il QSO",
        "Nadane TU zapisuje QSO",
        "Переданное TU записывает QSO",
    ]),
    ("Keyer", ["Keyer", "Keyer", "Keyer", "Keyer", "Keyer", "Ключ"]),
    ("Swap paddles", ["Inverser les palettes", "Invertir paddles", "Inverter paddles", "Inverti le palette", "Zamień łopatki", "Поменять точки и тире"]),
    ("Press a key", ["Appuyez sur une touche", "Pulsa una tecla", "Pressione uma tecla", "Premi un tasto", "Naciśnij klawisz", "Нажмите клавишу"]),
    ("Ctrl keys the paddle now. Use Alt for the shortcuts.", [
        "Ctrl manipule maintenant. Utilisez Alt pour les raccourcis.",
        "Ctrl ahora manipula. Usa Alt para los atajos.",
        "Ctrl agora manipula. Use Alt para os atalhos.",
        "Ora Ctrl manipola. Usa Alt per le scorciatoie.",
        "Ctrl teraz nadaje. Skróty przez Alt.",
        "Ctrl теперь работает ключом. Для сочетаний клавиш используйте Alt.",
    ]),
    ("Vail adapter", ["Adaptateur Vail", "Adaptador Vail", "Adaptador Vail", "Adattatore Vail", "Adapter Vail", "Адаптер Vail"]),
    ("Search again", ["Chercher à nouveau", "Buscar de nuevo", "Procurar de novo", "Cerca di nuovo", "Szukaj ponownie", "Искать снова"]),
    ("Its own keyer", ["Son propre keyer", "Su propio keyer", "Keyer próprio", "Keyer proprio", "Własny keyer", "Собственный ключ"]),
    ("MIDI error", ["Erreur MIDI", "Error MIDI", "Erro MIDI", "Errore MIDI", "Błąd MIDI", "Ошибка MIDI"]),
    ("No MIDI output found. Plug the adapter in.", [
        "Aucune sortie MIDI trouvée. Branchez l'adaptateur.",
        "No se encontró ninguna salida MIDI. Conecta el adaptador.",
        "Nenhuma saída MIDI encontrada. Conecte o adaptador.",
        "Nessuna uscita MIDI trovata. Collega l'adattatore.",
        "Nie znaleziono wyjścia MIDI. Podłącz adapter.",
        "MIDI-выход не найден. Подключите адаптер.",
    ]),
    ("The adapter only reports the paddles and this program keys. Needs firmware 5.1 or newer. The adapter keeps it.", [
        "L'adaptateur signale seulement les palettes et ce programme manipule. Exige le firmware 5.1 ou plus récent. L'adaptateur le garde en mémoire.",
        "El adaptador solo indica los paddles y este programa manipula. Necesita firmware 5.1 o más reciente. El adaptador lo guarda.",
        "O adaptador só informa os paddles e este programa manipula. Precisa do firmware 5.1 ou mais novo. O adaptador guarda isso.",
        "L'adattatore segnala solo le palette e questo programma manipola. Richiede il firmware 5.1 o più recente. L'adattatore lo memorizza.",
        "Adapter zgłasza tylko łopatki, a kluczuje ten program. Wymaga firmware 5.1 lub nowszego. Adapter to zapamiętuje.",
        "Адаптер только сообщает о лепестках, а ключом работает эта программа. Нужна прошивка 5.1 или новее. Адаптер это запоминает.",
    ]),
    ("Back to the adapter's iambic B, for older firmware. The keyer here becomes Straight key.", [
        "Retour à l'Iambic B de l'adaptateur, pour un firmware plus ancien. Le keyer ici devient Pioche.",
        "Vuelta al Iambic B del adaptador, para firmware antiguo. El keyer aquí pasa a Manipulador vertical.",
        "Volta ao Iambic B do adaptador, para firmware antigo. O keyer aqui passa a Manipulador vertical.",
        "Ritorno all'Iambic B dell'adattatore, per firmware più vecchi. Il keyer qui diventa Tasto verticale.",
        "Powrót do Iambic B w adapterze, dla starszego firmware. Keyer tutaj zmienia się na Klucz sztorcowy.",
        "Возврат к Iambic B адаптера, для старой прошивки. Ключ здесь становится Простой ключ.",
    ]),
    ("Passthrough is set. Key both paddles to check.", [
        "Le passthrough est activé. Manipulez les deux palettes pour vérifier.",
        "El passthrough está activo. Manipula ambos paddles para comprobarlo.",
        "O passthrough está ativo. Manipule os dois paddles para conferir.",
        "Il passthrough è attivo. Manipola entrambe le palette per controllare.",
        "Passthrough ustawiony. Na próbę użyj obu łopatek.",
        "Passthrough включён. Для проверки поработайте обоими лепестками.",
    ]),
    ("The adapter keys by itself again, the keyer here is Straight key.", [
        "L'adaptateur manipule de nouveau lui-même, le keyer ici est Pioche.",
        "El adaptador vuelve a manipular por sí mismo, el keyer aquí es Manipulador vertical.",
        "O adaptador volta a manipular sozinho, o keyer aqui é Manipulador vertical.",
        "L'adattatore manipola di nuovo da solo, il keyer qui è Tasto verticale.",
        "Adapter znów kluczuje sam, keyer tutaj to Klucz sztorcowy.",
        "Адаптер снова работает ключом сам, ключ здесь Простой ключ.",
    ]),
    ("Shared run", ["Session partagée", "Sesión compartida", "Sessão compartilhada", "Sessione condivisa", "Wspólna sesja", "Общий сеанс"]),
    ("This run", ["Cette session", "Esta sesión", "Esta sessão", "Questa sessione", "Ta sesja", "Этот сеанс"]),
    ("Enter a code to work the same stations under the same conditions.", [
        "Entrez un code pour contacter les mêmes stations dans les mêmes conditions.",
        "Introduce un código para trabajar las mismas estaciones en las mismas condiciones.",
        "Digite um código para trabalhar as mesmas estações nas mesmas condições.",
        "Inserisci un codice per collegare le stesse stazioni nelle stesse condizioni.",
        "Wpisz kod, aby zrobić te same stacje w tych samych warunkach.",
        "Введите код, чтобы провести те же станции в тех же условиях.",
    ]),
    ("Start", ["Lancer", "Iniciar", "Iniciar", "Avvia", "Start", "Старт"]),
    ("That is not a run code.", ["Ce n'est pas un code de session.", "Eso no es un código de sesión.", "Isso não é um código de sessão.", "Questo non è un codice di sessione.", "To nie jest kod sesji.", "Это не код сеанса."]),
    ("A character is wrong. Check the code.", ["Un caractère est faux. Vérifiez le code.", "Un carácter es incorrecto. Revisa el código.", "Um caractere está errado. Confira o código.", "Un carattere è sbagliato. Controlla il codice.", "Jeden znak jest błędny. Sprawdź kod.", "Неверный символ. Проверьте код."]),
    ("Code from simulation version", ["Code de la version de simulation", "Código de la versión de simulación", "Código da versão de simulação", "Codice della versione di simulazione", "Kod z wersji symulacji", "Код версии симуляции"]),
    ("this program has", ["ce programme a", "este programa tiene", "este programa tem", "questo programma ha", "ten program ma", "у этой программы"]),
    ("The code holds a value this program can't use.", [
        "Le code contient une valeur que ce programme ne peut pas utiliser.",
        "El código contiene un valor que este programa no puede usar.",
        "O código contém um valor que este programa não pode usar.",
        "Il codice contiene un valore che questo programma non può usare.",
        "Kod zawiera wartość, której ten program nie obsługuje.",
        "Код содержит значение, которое эта программа не поддерживает.",
    ]),
    ("Recording and keyer", ["Enregistrement et keyer", "Grabación y keyer", "Gravação e keyer", "Registrazione e keyer", "Nagrywanie i keyer", "Запись и ключ"]),
    ("Record each run", ["Enregistrer chaque session", "Grabar cada sesión", "Gravar cada sessão", "Registra ogni sessione", "Nagrywaj każdą sesję", "Записывать каждый сеанс"]),
    ("Play last recording", ["Lire le dernier enregistrement", "Reproducir la última grabación", "Tocar a última gravação", "Riproduci l'ultima registrazione", "Odtwórz ostatnie nagranie", "Воспроизвести последнюю запись"]),
    ("An external keyer sends the call", [
        "Un keyer externe envoie l'indicatif",
        "Un keyer externo envía el indicativo",
        "Um keyer externo envia o indicativo",
        "Un keyer esterno invia il nominativo",
        "Zewnętrzny keyer nadaje znak",
        "Внешний ключ передаёт позывной",
    ]),
    ("Display", ["Affichage", "Pantalla", "Exibição", "Visualizzazione", "Wyświetlanie", "Отображение"]),
    ("Language", ["Langue", "Idioma", "Idioma", "Lingua", "Język", "Язык"]),
    ("Colours", ["Couleurs", "Colores", "Cores", "Colori", "Kolory", "Цвета"]),
    ("Morsehunter dark", ["Morsehunter sombre", "Morsehunter oscuro", "Morsehunter escuro", "Morsehunter scuro", "Morsehunter ciemny", "Morsehunter тёмная"]),
    ("Morsehunter light", ["Morsehunter clair", "Morsehunter claro", "Morsehunter claro", "Morsehunter chiaro", "Morsehunter jasny", "Morsehunter светлая"]),
    ("DL8UG light", ["DL8UG clair", "DL8UG claro", "DL8UG claro", "DL8UG chiaro", "DL8UG jasny", "DL8UG светлая"]),
    ("DL8UG dark", ["DL8UG sombre", "DL8UG oscuro", "DL8UG escuro", "DL8UG scuro", "DL8UG ciemny", "DL8UG тёмная"]),
    ("High contrast", ["Contraste élevé", "Alto contraste", "Alto contraste", "Contrasto elevato", "Wysoki kontrast", "Высокий контраст"]),
    ("Show what is sent in a tab beside the log", [
        "Afficher les émissions dans un onglet à côté du log",
        "Mostrar lo transmitido en una pestaña junto al log",
        "Mostrar o que é transmitido numa aba ao lado do log",
        "Mostra ciò che viene trasmesso in una scheda accanto al log",
        "Pokaż nadawane w karcie obok logu",
        "Показывать переданное во вкладке рядом с логом",
    ]),
    ("It gives away every call. A run with it on at any time scores nothing and stays out of the high scores.", [
        "Cela révèle chaque indicatif. Une session où il a été actif à un moment ne marque aucun point et n'entre pas au classement.",
        "Revela cada indicativo. Una sesión en la que estuvo activo en algún momento no puntúa y no entra en la clasificación.",
        "Revela cada indicativo. Uma sessão em que esteve ativo em algum momento não pontua e fica fora da classificação.",
        "Rivela ogni nominativo. Una sessione in cui è stato attivo anche solo un momento non fa punti e resta fuori dalla classifica.",
        "Zdradza każdy znak. Sesja, w której było to kiedykolwiek włączone, nie dostaje punktów ani miejsca w rankingu.",
        "Это раскрывает каждый позывной. Сеанс, в котором это хоть раз было включено, не получает очков и не попадает в рейтинг.",
    ]),
    ("Audio dropouts", ["Coupures audio", "Cortes de audio", "Falhas de áudio", "Interruzioni audio", "Przerwy dźwięku", "Пропуски звука"]),
    ("Analysis", ["Analyse", "Análisis", "Análise", "Analisi", "Analiza", "Разбор"]),
    ("Progress", ["Progression", "Progreso", "Progresso", "Progressi", "Postępy", "Прогресс"]),
    ("Activity comes in waves", ["L'activité varie", "La actividad varía", "A atividade varia", "L'attività varia", "Aktywność się zmienia", "Активность меняется"]),
    ("Callers come in waves, with lulls and short openings, as in a real contest. Not in the original.", ["Les stations arrivent par vagues, avec des creux et de courtes ouvertures, comme dans un vrai concours. Absent de l'original.", "Las estaciones llegan en oleadas, con calmas y aperturas cortas, como en un concurso real. No está en el original.", "As estações chegam em ondas, com calmarias e aberturas curtas, como num concurso real. Não existe no original.", "Le stazioni arrivano a ondate, con pause e brevi aperture, come in una gara vera. Non c'è nell'originale.", "Stacje przychodzą falami, z przestojami i krótkimi otwarciami, jak w prawdziwych zawodach. Nie ma tego w oryginale.", "Станции приходят волнами, с затишьями и короткими прохождениями, как в настоящем контесте. В оригинале этого нет."]),
    ("Copying", ["Réception", "Copia", "Cópia", "Ricezione", "Odbiór", "Приём"]),
    ("Log a few contacts and your copying errors show here.", ["Enregistrez quelques QSO et vos erreurs de réception s'affichent ici.", "Registra unos QSO y aquí aparecen tus errores de copia.", "Registre alguns QSOs e aqui aparecem seus erros de cópia.", "Registra qualche QSO e qui compaiono i tuoi errori di ricezione.", "Zapisz kilka QSO, a tutaj pojawią się twoje błędy odbioru.", "Проведите несколько связей, и здесь появятся ваши ошибки приёма."]),
    ("clean", ["corrects", "correctos", "corretos", "corretti", "czystych", "чистых"]),
    ("with an error", ["avec erreur", "con error", "com erro", "con errore", "z błędem", "с ошибкой"]),
    ("dupes", ["doublons", "duplicados", "duplicados", "doppi", "duplikatów", "повторов"]),
    ("No copying error so far.", ["Aucune erreur de réception pour l'instant.", "Ningún error de copia por ahora.", "Nenhum erro de cópia até agora.", "Nessun errore di ricezione finora.", "Na razie bez błędów odbioru.", "Пока ни одной ошибки приёма."]),
    ("no caller with a call like it", ["aucune station avec un indicatif proche", "ninguna estación con un indicativo parecido", "nenhuma estação com indicativo parecido", "nessuna stazione con un nominativo simile", "żadna stacja nie miała podobnego znaku", "ни у кого из зовущих нет похожего позывного"]),
    ("instead of", ["au lieu de", "en vez de", "em vez de", "invece di", "zamiast", "вместо"]),
    ("Mixed up", ["Confondus", "Confundidos", "Confundidos", "Scambiati", "Pomylone", "Перепутано"]),
    ("missing", ["manque", "falta", "falta", "manca", "brak", "пропущено"]),
    ("too many", ["en trop", "de más", "a mais", "in più", "za dużo", "лишнее"]),
    ("Keying", ["Manipulation", "Manipulación", "Manipulação", "Manipolazione", "Nadawanie", "Передача"]),
    ("Key a run with the paddle and this shows how clean your rhythm is.", ["Manipulez une session au paddle et vous verrez ici si votre rythme est propre.", "Transmite una sesión con el paddle y aquí verás lo limpio que es tu ritmo.", "Transmita uma sessão com o paddle e aqui você vê o quanto seu ritmo é limpo.", "Trasmetti una sessione con il paddle e qui vedrai quanto è pulito il tuo ritmo.", "Nadaj sesję kluczem, a tutaj zobaczysz, jak czysty jest twój rytm.", "Передайте сеанс манипулятором, и здесь будет видно, насколько чист ваш ритм."]),
    ("With a keyer only the pauses between letters and words are yours.", ["Avec un keyer, seules les pauses entre lettres et mots viennent de vous.", "Con keyer solo las pausas entre letras y palabras son tuyas.", "Com keyer só as pausas entre letras e palavras são suas.", "Con il keyer solo le pause tra lettere e parole sono tue.", "Z keyerem twoje są tylko przerwy między literami i słowami.", "С ключом-автоматом ваши только паузы между буквами и словами."]),
    ("dits", ["points", "puntos", "pontos", "punti", "kropki", "точки"]),
    ("book", ["norme", "norma", "padrão", "norma", "wzorzec", "норма"]),
    ("Dit", ["Point", "Punto", "Ponto", "Punto", "Kropka", "Точка"]),
    ("Dah", ["Trait", "Raya", "Traço", "Linea", "Kreska", "Тире"]),
    ("Dah to dit", ["Trait sur point", "Raya a punto", "Traço por ponto", "Linea su punto", "Kreska do kropki", "Тире к точке"]),
    ("Gap inside a letter", ["Pause dans la lettre", "Pausa dentro de la letra", "Pausa dentro da letra", "Pausa nella lettera", "Przerwa w znaku", "Пауза внутри знака"]),
    ("Steadiness", ["Régularité", "Regularidad", "Regularidade", "Regolarità", "Równość", "Ровность"]),
    ("Letter gap", ["Pause entre lettres", "Pausa entre letras", "Pausa entre letras", "Pausa tra lettere", "Przerwa między literami", "Пауза между буквами"]),
    ("Word gap", ["Pause entre mots", "Pausa entre palabras", "Pausa entre palavras", "Pausa tra parole", "Przerwa między słowami", "Пауза между словами"]),
    ("Unreadable", ["Illisible", "Ilegible", "Ilegível", "Illeggibile", "Nieczytelne", "Нечитаемо"]),
    ("of", ["sur", "de", "de", "su", "z", "из"]),
    ("characters", ["caractères", "caracteres", "caracteres", "caratteri", "znaków", "знаков"]),
    ("Error signs", ["Signes d'erreur", "Signos de error", "Sinais de erro", "Segni di errore", "Znaki błędu", "Знаки ошибки"]),
    ("Clean keying, nothing stands out.", ["Manipulation propre, rien ne ressort.", "Manipulación limpia, nada destaca.", "Manipulação limpa, nada chama a atenção.", "Manipolazione pulita, niente salta all'occhio.", "Czyste nadawanie, nic się nie wyróżnia.", "Чистая передача, ничего не выделяется."]),
    ("Your letters run together. Leave more room between them.", ["Vos lettres se touchent. Laissez plus de place entre elles.", "Tus letras se juntan. Deja más espacio entre ellas.", "Suas letras se juntam. Deixe mais espaço entre elas.", "Le tue lettere si attaccano. Lascia più spazio tra di loro.", "Twoje litery zlewają się. Zostaw między nimi więcej miejsca.", "Ваши буквы сливаются. Оставляйте между ними больше места."]),
    ("Your letters stand far apart. That makes the over slow.", ["Vos lettres sont très espacées. Cela ralentit l'émission.", "Tus letras están muy separadas. Eso hace lenta la pasada.", "Suas letras ficam muito separadas. Isso deixa a transmissão lenta.", "Le tue lettere sono molto distanti. Questo rallenta la trasmissione.", "Twoje litery stoją daleko od siebie. To spowalnia nadawanie.", "Ваши буквы стоят далеко друг от друга. Это замедляет передачу."]),
    ("Your words hardly stand apart from the letters. Pause longer after a word.", ["Vos mots se distinguent à peine des lettres. Faites une pause plus longue après un mot.", "Tus palabras apenas se distinguen de las letras. Haz una pausa más larga tras una palabra.", "Suas palavras mal se distinguem das letras. Faça uma pausa mais longa depois de uma palavra.", "Le tue parole si distinguono appena dalle lettere. Fai una pausa più lunga dopo una parola.", "Twoje słowa ledwo odróżniają się od liter. Po słowie rób dłuższą przerwę.", "Ваши слова почти не отделяются от букв. Делайте после слова паузу длиннее."]),
    ("Your dahs are short against the dits.", ["Vos traits sont courts par rapport aux points.", "Tus rayas son cortas frente a los puntos.", "Seus traços são curtos em relação aos pontos.", "Le tue linee sono corte rispetto ai punti.", "Twoje kreski są krótkie w stosunku do kropek.", "Ваши тире коротки по сравнению с точками."]),
    ("Your dahs are long against the dits.", ["Vos traits sont longs par rapport aux points.", "Tus rayas son largas frente a los puntos.", "Seus traços são longos em relação aos pontos.", "Le tue linee sono lunghe rispetto ai punti.", "Twoje kreski są długie w stosunku do kropek.", "Ваши тире длинны по сравнению с точками."]),
    ("Your dits and dahs vary a lot in length. Aim for an even hand.", ["Vos points et traits varient beaucoup en longueur. Visez une main régulière.", "Tus puntos y rayas varían mucho de longitud. Busca una mano regular.", "Seus pontos e traços variam muito no comprimento. Procure uma mão regular.", "I tuoi punti e linee variano molto in lunghezza. Cerca una mano regolare.", "Twoje kropki i kreski mocno różnią się długością. Staraj się o równą rękę.", "Ваши точки и тире сильно гуляют по длине. Старайтесь держать ровную руку."]),
    ("Many characters were no Morse character. Watch the gaps inside the letters.", ["Beaucoup de caractères n'étaient pas du Morse. Surveillez les pauses dans les lettres.", "Muchos caracteres no eran Morse. Cuida las pausas dentro de las letras.", "Muitos caracteres não eram Morse. Cuide das pausas dentro das letras.", "Molti caratteri non erano Morse. Cura le pause dentro le lettere.", "Wiele znaków nie było znakami Morse'a. Uważaj na przerwy wewnątrz liter.", "Многие знаки не были знаками Морзе. Следите за паузами внутри букв."]),
    ("Best skill points per hour of each day", ["Meilleurs points de performance par heure de chaque jour", "Mejores puntos de rendimiento por hora de cada día", "Melhores pontos de desempenho por hora de cada dia", "Migliori punti di rendimento all'ora di ogni giorno", "Najlepsze punkty sprawności na godzinę z każdego dnia", "Лучшие очки мастерства в час за каждый день"]),
    ("runs", ["sessions", "sesiones", "sessões", "sessioni", "sesji", "сеансов"]),
    ("A few days of runs and your progress shows here.", ["Après quelques jours de sessions, votre progression s'affiche ici.", "Tras unos días de sesiones aquí verás tu progreso.", "Depois de alguns dias de sessões, aqui aparece seu progresso.", "Dopo qualche giorno di sessioni qui vedrai i tuoi progressi.", "Po kilku dniach sesji zobaczysz tu swoje postępy.", "Через несколько дней сеансов здесь будет виден ваш прогресс."]),
    ("Best day", ["Meilleur jour", "Mejor día", "Melhor dia", "Giorno migliore", "Najlepszy dzień", "Лучший день"]),
    ("days", ["jours", "días", "dias", "giorni", "dni", "дней"]),
];

fn index(lang: Lang) -> Option<usize> {
    match lang {
        Lang::En | Lang::De => None,
        Lang::Fr => Some(0),
        Lang::Es => Some(1),
        Lang::Pt => Some(2),
        Lang::It => Some(3),
        Lang::Pl => Some(4),
        Lang::Ru => Some(5),
    }
}

/// The text in `lang`, English if the table does not have it.
pub fn lookup(lang: Lang, en: &'static str) -> &'static str {
    static MAP: OnceLock<HashMap<&'static str, &'static [&'static str; 6]>> = OnceLock::new();
    let map = MAP.get_or_init(|| TABLE.iter().map(|(en, l)| (*en, l)).collect());
    match (index(lang), map.get(en)) {
        (Some(i), Some(l)) => l[i],
        _ => en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every English text given to `tr` or to `t` in the window, from the sources.
    fn used() -> Vec<String> {
        let sources = [
            include_str!("mod.rs"),
            include_str!("main_view.rs"),
            include_str!("setup.rs"),
            include_str!("debug.rs"),
            include_str!("fields.rs"),
            include_str!("help.rs"),
            include_str!("analysis.rs"),
        ];
        let mut out = Vec::new();
        for src in sources {
            let b = src.as_bytes();
            let mut i = 0;
            while i < b.len() {
                let word = |w: &str| b[i..].starts_with(w.as_bytes()) && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'));
                let start = if word("tr(") {
                    // tr(lang, "..."
                    src[i..].find(',').map(|c| i + c + 1)
                } else if word("t(") {
                    Some(i + 2)
                } else {
                    None
                };
                if let Some(mut j) = start {
                    while j < b.len() && b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < b.len() && b[j] == b'"' {
                        let mut k = j + 1;
                        while b[k] != b'"' {
                            k += if b[k] == b'\\' { 2 } else { 1 };
                        }
                        out.push(src[j + 1..k].replace("\\\"", "\""));
                    }
                }
                i += 1;
            }
        }
        out
    }

    #[test]
    fn every_text_of_the_window_is_translated() {
        let used = used();
        assert!(used.len() > 100, "the scan finds the texts, got {}", used.len());
        let missing: Vec<_> = used.iter().filter(|en| !TABLE.iter().any(|(e, _)| e == en)).collect();
        assert!(missing.is_empty(), "no translation for {missing:#?}");
    }

    #[test]
    fn the_table_has_no_leftovers_or_doubles() {
        let used = used();
        let stale: Vec<_> = TABLE.iter().map(|(e, _)| *e).filter(|e| !used.iter().any(|u| u == e)).collect();
        assert!(stale.is_empty(), "no longer used: {stale:#?}");
        for (n, (e, l)) in TABLE.iter().enumerate() {
            assert!(!TABLE[..n].iter().any(|(f, _)| f == e), "twice: {e}");
            assert!(l.iter().all(|t| !t.is_empty()), "empty: {e}");
        }
    }

    #[test]
    fn languages_fall_back_and_look_up() {
        assert_eq!(lookup(Lang::Fr, "Help"), "Aide");
        assert_eq!(lookup(Lang::Ru, "Help"), "Справка");
        assert_eq!(lookup(Lang::Pl, "not in the table"), "not in the table");
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Lang::Pt);
        assert_eq!(Lang::from_locale("ru_RU.UTF-8"), Lang::Ru);
        assert_eq!(Lang::from_locale("C"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }
}
