// corpus: accept/a34-wide-array-literal
// purpose: Builds array literals wider than the temporaries bound, with constant and computed elements.
// rule: C6
// phase: P2.2
// tsc: accepts
// js-comparable: yes
function twice(n: number): number {
  return n * 2;
}

export function main(): void {
  const table: number[] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159,160,161,162,163,164,165,166,167,168,169,170,171,172,173,174,175,176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191,192,193,194,195,196,197,198,199];
  const computed: number[] = [twice(0),twice(1),twice(2),twice(3),twice(4),twice(5),twice(6),twice(7),twice(8),twice(9),twice(10),twice(11),twice(12),twice(13),twice(14),twice(15),twice(16),twice(17),twice(18),twice(19),twice(20),twice(21),twice(22),twice(23),twice(24),twice(25),twice(26),twice(27),twice(28),twice(29),twice(30),twice(31),twice(32),twice(33),twice(34),twice(35),twice(36),twice(37),twice(38),twice(39),twice(40),twice(41),twice(42),twice(43),twice(44),twice(45),twice(46),twice(47),twice(48),twice(49),twice(50),twice(51),twice(52),twice(53),twice(54),twice(55),twice(56),twice(57),twice(58),twice(59),twice(60),twice(61),twice(62),twice(63),twice(64),twice(65),twice(66),twice(67),twice(68),twice(69),twice(70),twice(71),twice(72),twice(73),twice(74),twice(75),twice(76),twice(77),twice(78),twice(79),twice(80),twice(81),twice(82),twice(83),twice(84),twice(85),twice(86),twice(87),twice(88),twice(89),twice(90),twice(91),twice(92),twice(93),twice(94),twice(95),twice(96),twice(97),twice(98),twice(99),twice(100),twice(101),twice(102),twice(103),twice(104),twice(105),twice(106),twice(107),twice(108),twice(109),twice(110),twice(111),twice(112),twice(113),twice(114),twice(115),twice(116),twice(117),twice(118),twice(119),twice(120),twice(121),twice(122),twice(123),twice(124),twice(125),twice(126),twice(127),twice(128),twice(129),twice(130),twice(131),twice(132),twice(133),twice(134),twice(135),twice(136),twice(137),twice(138),twice(139),twice(140),twice(141),twice(142),twice(143),twice(144),twice(145),twice(146),twice(147),twice(148),twice(149)];
  let total = 0;
  for (const v of table) {
    total += v;
  }
  for (const v of computed) {
    total += v;
  }
  print(`${table.length} ${computed.length} ${total} ${table[199]} ${computed[149]}`);
}
