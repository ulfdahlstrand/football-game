// Utökad lagdata — spelarstilar, styrkor/svagheter, stjärnspelare
// Kompletterar roster.json med visuell presentationsdata

const TEAM_EXTENDED = {
  'aurora-fc': {
    style: 'Aggressiv & Direkt',
    styleIcon: '⚡',
    strengths: ['Stark i press', 'Duellvinnare', 'Snabba kontringar'],
    weaknesses: ['Sårbar bakåt', 'Tappar energi sent'],
    star: { name: 'Einar Lund', role: 'fwd', trait: 'Risktagaren' },
    formation: '4-1-3',
    rating: { attack: 85, defense: 68, speed: 78, stamina: 72 }
  },
  'eclipse-town': {
    style: 'Teknisk & Bollkontroll',
    styleIcon: '🌙',
    strengths: ['Högt bollinnehav', 'Precisa passningar', 'Taktisk disciplin'],
    weaknesses: ['Långsam i omställning', 'Få direkta skott'],
    star: { name: 'Luna Eriksson', role: 'mid', trait: 'Spelföraren' },
    formation: '4-3-2',
    rating: { attack: 72, defense: 78, speed: 65, stamina: 82 }
  },
  'forge-fc': {
    style: 'Fysisk & Hårdspelad',
    styleIcon: '🔨',
    strengths: ['Starka i luften', 'Robust försvar', 'Dödbollsspecialister'],
    weaknesses: ['Begränsad teknik', 'Svag på ytor'],
    star: { name: 'Viktor Holm', role: 'def', trait: 'Klippan' },
    formation: '5-3-1',
    rating: { attack: 65, defense: 90, speed: 60, stamina: 85 }
  },
  'glacier-fc': {
    style: 'Defensiv & Kontring',
    styleIcon: '❄️',
    strengths: ['Kompakt block', 'Snabba kontringar', 'Mentalt starka'],
    weaknesses: ['Låg anfallsvolym', 'Svårt mot press'],
    star: { name: 'Aino Mäkinen', role: 'gk', trait: 'Muren' },
    formation: '5-4-0',
    rating: { attack: 58, defense: 92, speed: 74, stamina: 80 }
  },
  'granite-athletic': {
    style: 'Balanserad & Solid',
    styleIcon: '🪨',
    strengths: ['Ingen tydlig svaghet', 'Bra lagkemi', 'Konsekvent'],
    weaknesses: ['Sällan spektakulärt', 'Svårt att avläsa'],
    star: { name: 'Bjorn Strand', role: 'mid', trait: 'Motorn' },
    formation: '4-2-3',
    rating: { attack: 74, defense: 74, speed: 72, stamina: 76 }
  },
  'mirage-sc': {
    style: 'Lömsk & Oförutsägbar',
    styleIcon: '👁️',
    strengths: ['Oväntade rörelser', 'Kreativa lösningar', 'Bra på set pieces'],
    weaknesses: ['Inkonsekvent', 'Tappar fokus'],
    star: { name: 'Omar Fahad', role: 'fwd', trait: 'Trollkarlen' },
    formation: '3-4-2',
    rating: { attack: 80, defense: 65, speed: 82, stamina: 68 }
  },
  'nebula-rangers': {
    style: 'Explosiv & Snabb',
    styleIcon: '✨',
    strengths: ['Rena löpningar', 'Hög press', 'Snabb omställning'],
    weaknesses: ['Tröttnar snabbt', 'Sårbar bakom backlinjen'],
    star: { name: 'Stella Nova', role: 'fwd', trait: 'Raketen' },
    formation: '4-2-4',
    rating: { attack: 88, defense: 60, speed: 92, stamina: 65 }
  },
  'phoenix-rovers': {
    style: 'Intensiv & Passionerad',
    styleIcon: '🔥',
    strengths: ['Aldrig ger upp', 'Starka i slutet', 'Inspirerade av mål'],
    weaknesses: ['Slarvig i uppspel', 'Disciplinproblem'],
    star: { name: 'Axel Brandt', role: 'fwd', trait: 'Eldsjälen' },
    formation: '4-3-3',
    rating: { attack: 82, defense: 65, speed: 80, stamina: 90 }
  },
  'tempest-united': {
    style: 'Vind & Precision',
    styleIcon: '🌪️',
    strengths: ['Precist långspel', 'Bra kantspel', 'Taktiskt flexibel'],
    weaknesses: ['Stel i närkamp', 'Svag på hörnar'],
    star: { name: 'Kai Lindqvist', role: 'mid', trait: 'Dirigenten' },
    formation: '4-1-4',
    rating: { attack: 76, defense: 72, speed: 84, stamina: 74 }
  }
};

window.TEAM_EXTENDED = TEAM_EXTENDED;
