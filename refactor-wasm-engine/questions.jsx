// Frågor för varje fotbollsplan - stigande svårighet
const MATCH_DATA = [
  {
    id: 'parken',
    name: 'Parkens Gräsmatta',
    level: 'Nybörjare',
    color: '#7fb069',
    team: 'aurora-fc',
    questions: [
      {
        q: 'Du får bollen fri på mittplan. En medspelare ropar och är helt omarkerad till höger. Vad gör du?',
        options: [
          'Driver bollen själv mot mål',
          'Passar till den omarkerade medspelaren',
          'Skjuter direkt från mittplan'
        ],
        correct: 1,
        why: 'Att se upp och hitta den fria medspelaren är grunden i lagspel. En passning skapar bättre chans än en lång soloraid.'
      },
      {
        q: 'En motspelare springer mot dig med bollen. Vad är bäst?',
        options: [
          'Kasta sig in i en tackling direkt',
          'Backa och hålla avstånd så hen inte kan passera',
          'Stå helt stilla'
        ],
        correct: 1,
        why: 'Att jockey:a (backa och hålla position) tvingar motståndaren att ta ett beslut. En ogenomtänkt tackling kan ge frispark.'
      },
      {
        q: 'Du har just förlorat bollen i anfall. Första reaktionen?',
        options: [
          'Gå tillbaka i lugn takt',
          'Pressa direkt för att återerövra',
          'Klaga på domaren'
        ],
        correct: 1,
        why: 'Motpressning de första 5 sekunderna ger störst chans att vinna tillbaka bollen – laget är redan högt upp i plan.'
      },
      {
        q: 'Din lagkamrat gör en miss. Vad säger du?',
        options: [
          '"Kom igen, nästa boll!"',
          '"Vad håller du på med?!"',
          'Säger ingenting alls'
        ],
        correct: 0,
        why: 'Positiv coachning håller laget upp. Kritik under match sänker självförtroendet och prestationen.'
      }
    ]
  },
  {
    id: 'kullen',
    name: 'Kullens IP',
    level: 'Lätt',
    color: '#5a9a4a',
    team: 'glacier-fc',
    questions: [
      {
        q: 'Du är innerback. Anfallaren fintar ut mot kanten. Var ställer du dig?',
        options: [
          'Följer tätt i kroppskontakt',
          'Styr hen mot sidlinjen, bort från mål',
          'Rusar fram och försöker snappa'
        ],
        correct: 1,
        why: 'Sidlinjen är din bästa försvarare. Att styra anfallaren utåt minskar skottvinklar och farliga ytor.'
      },
      {
        q: 'Frispark i farligt läge, 25 meter ut. Vad avgör om du ska skjuta själv?',
        options: [
          'Att du känner för det',
          'Vinkel, skyttestatistik och murens position',
          'Vem som ropar högst'
        ],
        correct: 1,
        why: 'Beslut bygger på data och situation, inte magkänsla. Analys av vinkel och mur avgör bästa valet.'
      },
      {
        q: 'Motståndaren har boll nära egen straffområde. Hur agerar din anfallslinje?',
        options: [
          'Delar upp sig och stänger passningsvägar',
          'Alla rusar på bollhållaren',
          'Går och ställer sig i offside'
        ],
        correct: 0,
        why: 'Samordnad press stänger passningar. Kaotisk jakt gör det lätt att spela sig ur pressen.'
      },
      {
        q: 'Du är trött i 85:e minuten. Vad prioriterar du?',
        options: [
          'Försöker ändå springa överallt',
          'Håller positionen och är smart med löpningar',
          'Går av planen'
        ],
        correct: 1,
        why: 'Positionsspel sparar energi. Smarta löpningar vid rätt tillfälle ger större effekt än konstant rörelse.'
      }
    ]
  },
  {
    id: 'arena',
    name: 'Norra Arenan',
    level: 'Medel',
    color: '#4a8d3a',
    team: 'nebula-rangers',
    questions: [
      {
        q: 'Ditt lag ligger under 0-1 med 10 minuter kvar. Tränaren byter in en extra anfallare. Hur justerar du?',
        options: [
          'Fortsätter exakt som innan',
          'Söker mer djupledslöpningar och tar risker',
          'Backar hem och försvarar'
        ],
        correct: 1,
        why: 'Taktisk anpassning är centralt. Med extra anfallare krävs högre press och mer aggressiva löpningar för att tvinga fram chanser.'
      },
      {
        q: 'En medspelare tacklas hårt och ligger kvar. Vad gör du först?',
        options: [
          'Spelar vidare om domaren inte blåser',
          'Sparkar ut bollen så hen får vård',
          'Springer och bråkar med motståndaren'
        ],
        correct: 1,
        why: 'Fair play – säkerhet först. Bollen lämnas tillbaka efter avbrottet och respekten består.'
      },
      {
        q: 'Du märker att högerbacken alltid är sen i uppspel. Hur använder du det?',
        options: [
          'Gör djupledslöpningar i ryggen på hen',
          'Ignorerar och spelar som vanligt',
          'Ropar åt domaren'
        ],
        correct: 0,
        why: 'Läsa motståndarens svagheter och utnyttja dem är en nyckelfärdighet. Djupledslöpningar i ryggen straffar sena backar.'
      },
      {
        q: 'Hörna för er. Du är kort och snabb. Var ska du stå?',
        options: [
          'Mitt i straffområdet bland de långa',
          'Vid främre stolpen för nick-avslut',
          'Utanför boxen för retur och andraboll'
        ],
        correct: 2,
        why: 'Roll och styrkor styr positionen. Snabba spelare skapar flest mål på returer utanför boxen, inte i luftdueller.'
      }
    ]
  },
  {
    id: 'stadion',
    name: 'Stadion',
    level: 'Svår',
    color: '#3d7a2e',
    team: 'tempest-united',
    questions: [
      {
        q: 'Ni spelar 4-3-3 mot ett lag i 3-5-2. Var finns era största ytor?',
        options: [
          'Centralt på mittplan',
          'I ryggen på deras wingbackar',
          'Inne i deras straffområde'
        ],
        correct: 1,
        why: 'Wingbackar lämnar ytor bakom sig när de går upp. Ytterforwards ska attackera dessa zoner för övertag.'
      },
      {
        q: 'Ditt lag pressar högt men motståndarens keeper slår långa bollar. Vad justerar ni?',
        options: [
          'Släpper pressen helt',
          'Flyttar upp backlinjen och matchar andrabollarna',
          'Byter ut keepern'
        ],
        correct: 1,
        why: 'Högt försvar kräver kompakthet. Att matcha andrabollar i mittfältet är nyckeln mot långbollstaktik.'
      },
      {
        q: 'Domaren gör ett felaktigt beslut som gynnar er. Hur reagerar du som kapten?',
        options: [
          'Tackar och spelar vidare tyst',
          'Säger till domaren hur det faktiskt var',
          'Firar högt med laget'
        ],
        correct: 1,
        why: 'Integritet överstiger resultat. Ärliga spelare bygger respekt hos domare och lag över tid.'
      },
      {
        q: 'Straffläggning. Du är femte skytt, 4-4. Vad fokuserar du på?',
        options: [
          'Keeperns rörelser precis före skottet',
          'Din egen process – punkt, placering, teknik',
          'Publikens tjut'
        ],
        correct: 1,
        why: 'Inre fokus på egen rutin skär bort yttre stress. Toppskyttar bestämmer placering innan de går fram.'
      }
    ]
  }
];

window.MATCH_DATA = MATCH_DATA;
