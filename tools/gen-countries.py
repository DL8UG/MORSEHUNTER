#!/usr/bin/env python3
r"""Builds crates/app/src/ui/countries.rs: the cty.dat names in de fr es pt it pl ru.
Countries from iso-codes (gettext, pt = pt_BR), fixed by hand; DXCC islands and regions by hand.
Needs the iso-codes package (/usr/share/iso-codes, /usr/share/locale).

    grep -oP '^\s*c\("[^"]*", "\K[^"]+' crates/sim/src/prefixes.rs > /tmp/names.txt
    python3 tools/gen-countries.py /tmp/names.txt crates/app/src/ui/countries.rs
"""
import gettext, json, sys

ISO = json.load(open("/usr/share/iso-codes/json/iso_3166-1.json"))["3166-1"]
BY = {e["alpha_2"]: e for e in ISO}
NAMES = [l.strip() for l in open(sys.argv[1])]
LANGS = [["de"], ["fr"], ["es"], ["pt_BR", "pt"], ["it"], ["pl"], ["ru"]]
TR = [gettext.translation("iso_3166-1", "/usr/share/locale", l) for l in LANGS]

ALIAS = {"St. Barthelemy": "BL", "St. Pierre & Miquelon": "PM", "St. Martin": "MF", "St. Lucia": "LC",
         "St. Vincent": "VC", "US Virgin Islands": "VI", "Sint Maarten": "SX", "St. Kitts & Nevis": "KN",
         "Curacao": "CW", "Falkland Islands": "FK", "Fed. Rep. of Germany": "DE", "Bosnia-Herzegovina": "BA",
         "Vatican City": "VA", "Aland Islands": "AX", "Palestine": "PS", "Republic of Korea": "KR",
         "DPR of Korea": "KP", "Timor - Leste": "TL", "Wallis & Futuna Islands": "WF", "Mariana Islands": "MP",
         "Micronesia": "FM", "Pitcairn Island": "PN", "Tokelau Islands": "TK", "Dem. Rep. of the Congo": "CD",
         "The Gambia": "GM", "Reunion Island": "RE", "Cote d'Ivoire": "CI", "St. Helena": "SH"}

# fixes to the iso-codes names: long official forms, typos, short forms for the column
FIX = {
    "United States": ["USA", "États-Unis", "EE. UU.", "EUA", "Stati Uniti", "USA", "США"],
    "St. Martin": ["Saint-Martin", "Saint-Martin", "San Martín (FR)", "São Martinho (FR)", "Saint-Martin", "Saint-Martin", "Сен-Мартен"],
    "Sint Maarten": ["Sint Maarten", "Saint-Martin (NL)", "San Martín (NL)", "São Martinho (NL)", "Sint Maarten", "Sint Maarten", "Синт-Мартен"],
    "St. Vincent": ["St. Vincent", "Saint-Vincent", "San Vicente", "São Vicente", "Saint Vincent", "Saint Vincent", "Сент-Винсент"],
    "US Virgin Islands": ["Amerikanische Jungferninseln", "Îles Vierges des États-Unis", "Islas Vírgenes de EE. UU.", "Ilhas Virgens Americanas", "Isole Vergini americane", "Wyspy Dziewicze USA", "Виргинские острова (США)"],
    "British Virgin Islands": ["Britische Jungferninseln", "Îles Vierges britanniques", "Islas Vírgenes Británicas", "Ilhas Virgens Britânicas", "Isole Vergini britanniche", "Brytyjskie Wyspy Dziewicze", "Британские Виргинские острова"],
    "Turks & Caicos Islands": ["Turks- und Caicosinseln", "Îles Turques-et-Caïques", "Islas Turcas y Caicos", "Ilhas Turcas e Caicos", "Isole Turks e Caicos", "Turks i Caicos", "Тёркс и Кайкос"],
    "Falkland Islands": ["Falklandinseln", "Îles Malouines", "Islas Malvinas", "Ilhas Malvinas", "Isole Falkland", "Falklandy", "Фолклендские острова"],
    "Vatican City": ["Vatikanstadt", "Vatican", "Vaticano", "Vaticano", "Città del Vaticano", "Watykan", "Ватикан"],
    "Aland Islands": ["Åland", "Îles Åland", "Islas Åland", "Ilhas Åland", "Isole Åland", "Wyspy Alandzkie", "Аландские острова"],
    "Palestine": ["Palästina", "Palestine", "Palestina", "Palestina", "Palestina", "Palestyna", "Палестина"],
    "Solomon Islands": ["Salomonen", "Îles Salomon", "Islas Salomón", "Ilhas Salomão", "Isole Salomone", "Wyspy Salomona", "Соломоновы Острова"],
    "Mariana Islands": ["Nördliche Marianen", "Îles Mariannes du Nord", "Islas Marianas del Norte", "Ilhas Marianas do Norte", "Isole Marianne Settentrionali", "Mariany Północne", "Северные Марианские острова"],
    "Micronesia": ["Mikronesien", "Micronésie", "Micronesia", "Micronésia", "Micronesia", "Mikronezja", "Микронезия"],
    "Cocos (Keeling) Islands": ["Kokosinseln", "Îles Cocos", "Islas Cocos", "Ilhas Cocos", "Isole Cocos", "Wyspy Kokosowe", "Кокосовые острова"],
    "Christmas Island": ["Weihnachtsinsel", "Île Christmas", "Isla de Navidad", "Ilha Christmas", "Isola di Natale", "Wyspa Bożego Narodzenia", "Остров Рождества"],
    "Norfolk Island": ["Norfolkinsel", "Île Norfolk", "Isla Norfolk", "Ilha Norfolk", "Isola Norfolk", "Wyspa Norfolk", "Остров Норфолк"],
    "Pitcairn Island": ["Pitcairninseln", "Îles Pitcairn", "Islas Pitcairn", "Ilhas Pitcairn", "Isole Pitcairn", "Pitcairn", "Острова Питкэрн"],
    "Dem. Rep. of the Congo": ["DR Kongo", "RD Congo", "RD del Congo", "RD do Congo", "RD del Congo", "DR Konga", "ДР Конго"],
    "Republic of the Congo": ["Kongo", "Congo", "Congo", "Congo", "Congo", "Kongo", "Конго"],
    "Comoros": ["Komoren", "Comores", "Comoras", "Comores", "Comore", "Komory", "Коморы"],
    "Reunion Island": ["Réunion", "La Réunion", "Reunión", "Reunião", "Riunione", "Reunion", "Реюньон"],
    "Central African Republic": ["Zentralafrika", "Centrafrique", "Rep. Centroafricana", "Rep. Centro-Africana", "Centrafrica", "Rep. Środkowoafrykańska", "ЦАР"],
    "Cote d'Ivoire": ["Elfenbeinküste", "Côte d'Ivoire", "Costa de Marfil", "Costa do Marfim", "Costa d'Avorio", "Wybrzeże Kości Słoniowej", "Кот-д’Ивуар"],
    "St. Helena": ["St. Helena", "Sainte-Hélène", "Santa Elena", "Santa Helena", "Sant'Elena", "Wyspa Świętej Heleny", "Остров Святой Елены"],
    "United Arab Emirates": ["VAE", "EAU", "EAU", "EAU", "EAU", "ZEA", "ОАЭ"],
    "Tunisia": ["Tunesien", "Tunisie", "Túnez", "Tunísia", "Tunisia", "Tunezja", "Тунис"],
    "Suriname": ["Suriname", "Suriname", "Surinam", "Suriname", "Suriname", "Surinam", "Суринам"],
    "Wallis & Futuna Islands": ["Wallis und Futuna", "Wallis-et-Futuna", "Wallis y Futuna", "Wallis e Futuna", "Wallis e Futuna", "Wallis i Futuna", "Уоллис и Футуна"],
    "Cabo Verde": ["Kap Verde", "Cap-Vert", "Cabo Verde", "Cabo Verde", "Capo Verde", "Wyspy Zielonego Przylądka", "Кабо-Верде"],
    "Brunei Darussalam": ["Brunei", "Brunei", "Brunéi", "Brunei", "Brunei", "Brunei", "Бруней"],
    "Dominican Republic": ["Dominikanische Republik", "République dominicaine", "República Dominicana", "República Dominicana", "Repubblica Dominicana", "Dominikana", "Доминиканская Республика"],
    "Kingdom of Eswatini": ["Eswatini", "Eswatini", "Esuatini", "Essuatíni", "Eswatini", "Eswatini", "Эсватини"],
    "Iran": ["Iran", "Iran", "Irán", "Irã", "Iran", "Iran", "Иран"],
    "Timor - Leste": ["Osttimor", "Timor oriental", "Timor Oriental", "Timor-Leste", "Timor Est", "Timor Wschodni", "Восточный Тимор"],
    "Sao Tome & Principe": ["São Tomé und Príncipe", "Sao Tomé-et-Principe", "Santo Tomé y Príncipe", "São Tomé e Príncipe", "São Tomé e Príncipe", "Wyspy Świętego Tomasza i Książęca", "Сан-Томе и Принсипи"],
    "Republic of South Sudan": ["Südsudan", "Soudan du Sud", "Sudán del Sur", "Sudão do Sul", "Sudan del Sud", "Sudan Południowy", "Южный Судан"],
    "Cayman Islands": ["Kaimaninseln", "Îles Caïmans", "Islas Caimán", "Ilhas Cayman", "Isole Cayman", "Kajmany", "Каймановы острова"],
    "Faroe Islands": ["Färöer", "Îles Féroé", "Islas Feroe", "Ilhas Faroé", "Isole Fær Øer", "Wyspy Owcze", "Фарерские острова"],
    "Papua New Guinea": ["Papua-Neuguinea", "Papouasie-Nouvelle-Guinée", "Papúa Nueva Guinea", "Papua-Nova Guiné", "Papua Nuova Guinea", "Papua-Nowa Gwinea", "Папуа — Новая Гвинея"],
}

# DXCC entities that are no country of their own
HAND = {
    "Sable Island": ["Sable Island", "Île de Sable", "Isla Sable", "Ilha Sable", "Isola Sable", "Wyspa Sable", "Остров Сейбл"],
    "St. Paul Island": ["St.-Paul-Insel", "Île Saint-Paul", "Isla San Pablo", "Ilha São Paulo", "Isola Saint Paul", "Wyspa Świętego Pawła", "Остров Сент-Пол"],
    "San Andres & Providencia": ["San Andrés", "San Andrés", "San Andrés y Providencia", "San Andrés", "San Andrés", "San Andrés", "Сан-Андрес"],
    "Guantanamo Bay": ["Guantánamo", "Guantánamo", "Guantánamo", "Guantánamo", "Guantánamo", "Guantánamo", "Гуантанамо"],
    "Alaska": ["Alaska", "Alaska", "Alaska", "Alasca", "Alaska", "Alaska", "Аляска"],
    "Navassa Island": ["Navassa", "Île de la Navase", "Isla de Navaza", "Ilha Navassa", "Isola Navassa", "Nawassa", "Остров Навасса"],
    "Desecheo Island": ["Desecheo", "Île Desecheo", "Isla Desecheo", "Ilha Desecheo", "Isola Desecheo", "Desecheo", "Остров Десечео"],
    "Saba & St. Eustatius": ["Saba", "Saba", "Saba", "Saba", "Saba", "Saba", "Саба"],
    "Cocos Island": ["Kokosinsel", "Île Cocos", "Isla del Coco", "Ilha do Coco", "Isola del Cocco", "Wyspa Kokosowa", "Остров Кокос"],
    "Revillagigedo": ["Revillagigedo", "Revillagigedo", "Revillagigedo", "Revillagigedo", "Revillagigedo", "Revillagigedo", "Ревилья-Хихедо"],
    "Aves Island": ["Isla Aves", "Île d'Aves", "Isla de Aves", "Ilha de Aves", "Isola Aves", "Wyspa Aves", "Остров Авес"],
    "San Felix & San Ambrosio": ["San Félix", "San Félix", "San Félix y San Ambrosio", "San Félix", "San Félix", "San Félix", "Сан-Феликс"],
    "Easter Island": ["Osterinsel", "Île de Pâques", "Isla de Pascua", "Ilha de Páscoa", "Isola di Pasqua", "Wyspa Wielkanocna", "Остров Пасхи"],
    "Juan Fernandez Islands": ["Juan Fernández", "Juan Fernández", "Juan Fernández", "Juan Fernández", "Juan Fernández", "Juan Fernández", "Хуан-Фернандес"],
    "Galapagos Islands": ["Galápagos", "Galápagos", "Galápagos", "Galápagos", "Galápagos", "Galapagos", "Галапагосские острова"],
    "Malpelo Island": ["Malpelo", "Malpelo", "Isla de Malpelo", "Ilha Malpelo", "Isola Malpelo", "Malpelo", "Мальпело"],
    "Bonaire": ["Bonaire", "Bonaire", "Bonaire", "Bonaire", "Bonaire", "Bonaire", "Бонайре"],
    "Fernando de Noronha": ["Fernando de Noronha", "Fernando de Noronha", "Fernando de Noronha", "Fernando de Noronha", "Fernando de Noronha", "Fernando de Noronha", "Фернанду-ди-Норонья"],
    "St. Peter & St. Paul": ["St. Peter und Paul", "Saint-Pierre-et-Saint-Paul", "San Pedro y San Pablo", "São Pedro e São Paulo", "San Pietro e San Paolo", "Skały Św. Piotra i Pawła", "Скалы Святого Петра и Павла"],
    "Trindade & Martim Vaz": ["Trindade", "Trindade", "Trindade", "Trindade e Martim Vaz", "Trindade", "Trindade", "Триндади"],
    "South Georgia Island": ["Südgeorgien", "Géorgie du Sud", "Georgia del Sur", "Geórgia do Sul", "Georgia del Sud", "Georgia Południowa", "Южная Георгия"],
    "South Shetland Islands": ["Südliche Shetlandinseln", "Shetland du Sud", "Shetland del Sur", "Shetland do Sul", "Shetland Meridionali", "Szetlandy Południowe", "Южные Шетландские острова"],
    "South Sandwich Islands": ["Südliche Sandwichinseln", "Sandwich du Sud", "Sandwich del Sur", "Sandwich do Sul", "Sandwich Australi", "Sandwich Południowy", "Южные Сандвичевы острова"],
    "Sov Mil Order of Malta": ["Malteserorden", "Ordre de Malte", "Orden de Malta", "Ordem de Malta", "Ordine di Malta", "Zakon Maltański", "Мальтийский орден"],
    "Azores": ["Azoren", "Açores", "Azores", "Açores", "Azzorre", "Azory", "Азорские острова"],
    "Balearic Islands": ["Balearen", "Baléares", "Islas Baleares", "Ilhas Baleares", "Baleari", "Baleary", "Балеарские острова"],
    "England": ["England", "Angleterre", "Inglaterra", "Inglaterra", "Inghilterra", "Anglia", "Англия"],
    "Northern Ireland": ["Nordirland", "Irlande du Nord", "Irlanda del Norte", "Irlanda do Norte", "Irlanda del Nord", "Irlandia Północna", "Северная Ирландия"],
    "Scotland": ["Schottland", "Écosse", "Escocia", "Escócia", "Scozia", "Szkocja", "Шотландия"],
    "Wales": ["Wales", "Pays de Galles", "Gales", "País de Gales", "Galles", "Walia", "Уэльс"],
    "Sardinia": ["Sardinien", "Sardaigne", "Cerdeña", "Sardenha", "Sardegna", "Sardynia", "Сардиния"],
    "Svalbard": ["Spitzbergen", "Svalbard", "Svalbard", "Svalbard", "Svalbard", "Svalbard", "Шпицберген"],
    "Jan Mayen": ["Jan Mayen", "Jan Mayen", "Jan Mayen", "Jan Mayen", "Jan Mayen", "Jan Mayen", "Ян-Майен"],
    "Market Reef": ["Märket", "Märket", "Märket", "Märket", "Märket", "Märket", "Мяркет"],
    "Franz Josef Land": ["Franz-Josef-Land", "Terre François-Joseph", "Tierra de Francisco José", "Terra de Francisco José", "Terra di Francesco Giuseppe", "Ziemia Franciszka Józefa", "Земля Франца-Иосифа"],
    "Dodecanese": ["Dodekanes", "Dodécanèse", "Dodecaneso", "Dodecaneso", "Dodecaneso", "Dodekanez", "Додеканес"],
    "Crete": ["Kreta", "Crète", "Creta", "Creta", "Creta", "Kreta", "Крит"],
    "Corsica": ["Korsika", "Corse", "Córcega", "Córsega", "Corsica", "Korsyka", "Корсика"],
    "European Russia": ["Russland (Europa)", "Russie (Europe)", "Rusia (Europa)", "Rússia (Europa)", "Russia (Europa)", "Rosja (Europa)", "Россия (Европа)"],
    "Asiatic Russia": ["Russland (Asien)", "Russie (Asie)", "Rusia (Asia)", "Rússia (Ásia)", "Russia (Asia)", "Rosja (Azja)", "Россия (Азия)"],
    "Kaliningrad": ["Kaliningrad", "Kaliningrad", "Kaliningrado", "Kaliningrado", "Kaliningrad", "Kaliningrad", "Калининград"],
    "Republic of Kosovo": ["Kosovo", "Kosovo", "Kosovo", "Kosovo", "Kosovo", "Kosowo", "Косово"],
    "Spratly Islands": ["Spratly-Inseln", "Îles Spratleys", "Islas Spratly", "Ilhas Spratly", "Isole Spratly", "Wyspy Spratly", "Острова Спратли"],
    "West Malaysia": ["Westmalaysia", "Malaisie occidentale", "Malasia Occidental", "Malásia Ocidental", "Malesia occidentale", "Malezja Zachodnia", "Западная Малайзия"],
    "East Malaysia": ["Ostmalaysia", "Malaisie orientale", "Malasia Oriental", "Malásia Oriental", "Malesia orientale", "Malezja Wschodnia", "Восточная Малайзия"],
    "Pratas Island": ["Pratas", "Îles Pratas", "Islas Pratas", "Ilhas Pratas", "Isole Pratas", "Pratas", "Острова Пратас"],
    "Ogasawara": ["Ogasawara", "Ogasawara", "Ogasawara", "Ogasawara", "Ogasawara", "Ogasawara", "Огасавара"],
    "Asiatic Turkey": ["Türkei", "Turquie", "Turquía", "Turquia", "Turchia", "Turcja", "Турция"],
    "Andaman & Nicobar Is.": ["Andamanen", "Andaman", "Andamán", "Andamão", "Andamane", "Andamany", "Андаманские острова"],
    "Lakshadweep Islands": ["Lakshadweep", "Laquedives", "Laquedivas", "Laquedivas", "Laccadive", "Lakszadiwy", "Лакшадвип"],
    "UK Base Areas on Cyprus": ["Brit. Basen auf Zypern", "Bases brit. de Chypre", "Bases brit. en Chipre", "Bases brit. em Chipre", "Basi brit. a Cipro", "Bazy bryt. na Cyprze", "Брит. базы на Кипре"],
    "South Cook Islands": ["Südliche Cookinseln", "Îles Cook du Sud", "Cook del Sur", "Cook do Sul", "Cook meridionali", "Południowe Wyspy Cooka", "Южные острова Кука"],
    "Temotu Province": ["Temotu", "Temotu", "Temotu", "Temotu", "Temotu", "Temotu", "Темоту"],
    "Baker & Howland Islands": ["Baker und Howland", "Baker et Howland", "Baker y Howland", "Baker e Howland", "Baker e Howland", "Baker i Howland", "Бейкер и Хауленд"],
    "Johnston Island": ["Johnston", "Johnston", "Atolón Johnston", "Atol Johnston", "Atollo Johnston", "Atol Johnston", "Джонстон"],
    "Midway Island": ["Midway", "Midway", "Midway", "Midway", "Midway", "Midway", "Мидуэй"],
    "Palmyra & Jarvis Islands": ["Palmyra", "Palmyra", "Palmyra", "Palmyra", "Palmyra", "Palmyra", "Пальмира"],
    "Hawaii": ["Hawaii", "Hawaï", "Hawái", "Havaí", "Hawaii", "Hawaje", "Гавайи"],
    "Kure Island": ["Kure", "Kure", "Kure", "Kure", "Kure", "Kure", "Куре"],
    "Wake Island": ["Wake", "Wake", "Isla Wake", "Ilha Wake", "Isola Wake", "Wake", "Уэйк"],
    "Western Kiribati": ["Westkiribati", "Kiribati occidental", "Kiribati Occidental", "Kiribati Ocidental", "Kiribati occidentale", "Kiribati Zachodnie", "Западное Кирибати"],
    "Central Kiribati": ["Zentralkiribati", "Kiribati central", "Kiribati Central", "Kiribati Central", "Kiribati centrale", "Kiribati Środkowe", "Центральное Кирибати"],
    "Eastern Kiribati": ["Ostkiribati", "Kiribati oriental", "Kiribati Oriental", "Kiribati Oriental", "Kiribati orientale", "Kiribati Wschodnie", "Восточное Кирибати"],
    "Banaba Island": ["Banaba", "Banaba", "Banaba", "Banaba", "Banaba", "Banaba", "Банаба"],
    "Lord Howe Island": ["Lord-Howe-Insel", "Île Lord Howe", "Isla Lord Howe", "Ilha Lord Howe", "Isola Lord Howe", "Lord Howe", "Лорд-Хау"],
    "Mellish Reef": ["Mellish Reef", "Récif Mellish", "Arrecife Mellish", "Recife Mellish", "Mellish Reef", "Rafa Mellish", "Риф Меллиш"],
    "Willis Island": ["Willis Island", "Île Willis", "Isla Willis", "Ilha Willis", "Isola Willis", "Wyspa Willis", "Остров Уиллис"],
    "Chatham Islands": ["Chatham-Inseln", "Îles Chatham", "Islas Chatham", "Ilhas Chatham", "Isole Chatham", "Wyspy Chatham", "Острова Чатем"],
    "Kermadec Islands": ["Kermadec-Inseln", "Îles Kermadec", "Islas Kermadec", "Ilhas Kermadec", "Isole Kermadec", "Wyspy Kermadec", "Острова Кермадек"],
    "N.Z. Subantarctic Is.": ["Subantarkt. Inseln NZ", "Îles subantarctiques NZ", "Islas subantárticas NZ", "Ilhas subantárticas NZ", "Isole subantartiche NZ", "Wyspy subantarktyczne NZ", "Субантарктика НЗ"],
    "Agalega & St. Brandon": ["Agaléga", "Agaléga", "Agaléga", "Agaléga", "Agaléga", "Agaléga", "Агалега"],
    "Rodriguez Island": ["Rodrigues", "Rodrigues", "Rodrigues", "Rodrigues", "Rodrigues", "Rodrigues", "Родригес"],
    "Annobon Island": ["Annobón", "Annobón", "Annobón", "Ano-Bom", "Annobón", "Annobón", "Аннобон"],
    "Madeira Islands": ["Madeira", "Madère", "Madeira", "Madeira", "Madera", "Madera", "Мадейра"],
    "Canary Islands": ["Kanaren", "Canaries", "Canarias", "Canárias", "Canarie", "Wyspy Kanaryjskie", "Канарские острова"],
    "Ceuta & Melilla": ["Ceuta und Melilla", "Ceuta et Melilla", "Ceuta y Melilla", "Ceuta e Melilla", "Ceuta e Melilla", "Ceuta i Melilla", "Сеута и Мелилья"],
    "Glorioso Islands": ["Glorieuses", "Glorieuses", "Islas Gloriosas", "Ilhas Gloriosas", "Isole Gloriose", "Wyspy Chwalebne", "Острова Глорьёз"],
    "Juan de Nova, Europa": ["Juan de Nova, Europa", "Juan de Nova, Europa", "Juan de Nova, Europa", "Juan de Nova, Europa", "Juan de Nova, Europa", "Juan de Nova, Europa", "Жуан-ди-Нова, Европа"],
    "Tromelin Island": ["Tromelin", "Tromelin", "Tromelin", "Tromelin", "Tromelin", "Tromelin", "Тромлен"],
    "Crozet Island": ["Crozetinseln", "Îles Crozet", "Islas Crozet", "Ilhas Crozet", "Isole Crozet", "Wyspy Crozeta", "Острова Крозе"],
    "Kerguelen Islands": ["Kerguelen", "Kerguelen", "Kerguelen", "Kerguelen", "Kerguelen", "Wyspy Kerguelena", "Кергелен"],
    "Amsterdam & St. Paul Is.": ["Amsterdam und St. Paul", "Amsterdam et Saint-Paul", "Ámsterdam y San Pablo", "Amsterdã e São Paulo", "Amsterdam e Saint-Paul", "Amsterdam i Saint-Paul", "Амстердам и Сен-Поль"],
    "Chagos Islands": ["Chagos-Archipel", "Chagos", "Chagos", "Chagos", "Chagos", "Czagos", "Чагос"],
    "Ascension Island": ["Ascension", "Ascension", "Ascensión", "Ascensão", "Ascensione", "Wyspa Wniebowstąpienia", "Остров Вознесения"],
    "Tristan da Cunha & Gough": ["Tristan da Cunha", "Tristan da Cunha", "Tristán de Acuña", "Tristão da Cunha", "Tristan da Cunha", "Tristan da Cunha", "Тристан-да-Кунья"],
    "Pr. Edward & Marion Is.": ["Marion-Insel", "Île Marion", "Isla Marion", "Ilha Marion", "Isola Marion", "Wyspa Mariona", "Остров Марион"],
}

idx = {}
for e in ISO:
    for k in ("name", "common_name", "official_name"):
        if k in e:
            idx[e[k].lower().replace("&", "and")] = e["alpha_2"]

rows, missing = [], []
for n in NAMES:
    if n in HAND:
        rows.append((n, HAND[n]))
    elif n in FIX:
        rows.append((n, FIX[n]))
    else:
        a = ALIAS.get(n) or idx.get(n.lower().replace("&", "and"))
        if not a:
            missing.append(n)
            continue
        e = BY[a]
        msg = e.get("common_name", e["name"])
        rows.append((n, [t.gettext(msg) for t in TR]))
# single cells: pt as in Brazil today
PATCH = {("Belarus", 3): "Bielorrússia", ("Eritrea", 3): "Eritreia", ("Burkina Faso", 3): "Burkina Faso", ("Burkina Faso", 2): "Burkina Faso"}
rows = [(n, [PATCH.get((n, i), x) for i, x in enumerate(v)]) for n, v in rows]
if missing:
    sys.exit("no names for: " + ", ".join(missing))
for n, v in rows:
    for x in v:
        if "," in x and n != "Juan de Nova, Europa":
            sys.exit(f"comma left in {n}: {x}")

CODE = """
use std::collections::HashMap;
use std::sync::OnceLock;

use mr_sim::country::short_name;
use mr_sim::prefixes::Country;
use mr_sim::settings::Lang;

/// The name of `c` for the log: English as cty.dat has it, made short, the rest from the table.
pub(super) fn name(c: &Country, lang: Lang) -> &'static str {
    let col = match lang {
        Lang::En => return short_name(c),
        Lang::De => 0,
        Lang::Fr => 1,
        Lang::Es => 2,
        Lang::Pt => 3,
        Lang::It => 4,
        Lang::Pl => 5,
        Lang::Ru => 6,
    };
    static MAP: OnceLock<HashMap<&'static str, &'static [&'static str; 7]>> = OnceLock::new();
    let map = MAP.get_or_init(|| NAMES.iter().map(|(en, all)| (*en, all)).collect());
    map.get(c.name).map_or_else(|| short_name(c), |all| all[col])
}

#[cfg(test)]
mod tests {
    use super::*;
    use mr_sim::prefixes::COUNTRIES;

    #[test]
    fn every_country_has_all_its_names() {
        let all: Vec<&Country> = COUNTRIES.iter().flat_map(|r| r.iter()).collect();
        for c in &all {
            let row = NAMES.iter().find(|(en, _)| *en == c.name).unwrap_or_else(|| panic!("no names for {}", c.name));
            assert!(row.1.iter().all(|n| !n.is_empty()), "{}", c.name);
        }
        assert_eq!(NAMES.len(), all.len(), "names for countries cty.dat does not have");
    }

    #[test]
    fn names_follow_the_language() {
        let de = mr_sim::country::of("DL8UG").unwrap();
        assert_eq!(name(de, Lang::En), "Germany");
        assert_eq!(name(de, Lang::De), "Deutschland");
        assert_eq!(name(de, Lang::Fr), "Allemagne");
        assert_eq!(name(de, Lang::Ru), "Германия");
        let ru = mr_sim::country::of("RA3AA").unwrap();
        assert_eq!(name(ru, Lang::Pl), "Rosja (Europa)");
    }
}"""

esc = lambda s: s.replace("\\", "\\\\").replace('"', '\\"')
out = ['//! The country names of cty.dat in the languages of the window, in the order',
       '//! de fr es pt it pl ru. Generated from the iso-codes translations (pt as in Brazil)',
       '//! with short forms for the log column; DXCC islands and regions by hand.',
       '', 'pub(super) const NAMES: &[(&str, [&str; 7])] = &[']
for n, v in rows:
    out.append(f'    ("{esc(n)}", [' + ", ".join(f'"{esc(x)}"' for x in v) + "]),")
out.append("];")
out.append(CODE)
open(sys.argv[2], "w").write("\n".join(out) + "\n")
print(len(rows), "rows")
