console.log("héllo".length, "😀".length, "日本".length);
console.log("￿" < "😀", "z" < "é");
console.log("é".toUpperCase(), "straße".toUpperCase(), "ÀB".toLowerCase());
console.log(JSON.stringify("  x　".trim()));
console.log(JSON.stringify("a,b,,c".split(",")), JSON.stringify("日本語".split("本")), JSON.stringify("hé".split("")));
console.log("héllo".includes("ll"), "héllo".includes(""), "héllo".startsWith("hé"), "héllo".endsWith("llo"), "héllo".startsWith("l", 2));
console.log(("\uD83D" + "\uDE00") === "😀");
console.log("a" + 1e-7, "a" + -0);
