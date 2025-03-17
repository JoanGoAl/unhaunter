import { marked } from "marked";
import { readFile, writeFile, mkdir, cp } from "fs/promises";
import Handlebars from "handlebars";
import { existsSync } from "fs";

async function mdToHtml() {
    const news = await readFile("./page/md/news.md", "utf8");

    return {
        news: marked(news)
    }
}

async function insertInfoHtml(obj) {
    const html = await readFile("./page/template.html")
    const result = Handlebars.compile(html.toString())(obj)
    return result
}

async function main() {
    try {

        const mdObjStrings = await mdToHtml()

        const newHtml = await insertInfoHtml(mdObjStrings);

        await writeFile("./dist/index.html", newHtml)
        await cp("./page/style.css", "./dist/style.css")

    } catch (error) {
        console.error("Error al leer el archivo:", error);
    }
}

main();
