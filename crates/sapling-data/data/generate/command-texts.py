"""BDS's English texts -> command-texts.tsv: the `commands.*` lines of en_US.lang, which BDS's console
prints a command's answer in. See ../../README.md "Command texts".

usage: py -3 command-texts.py
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
LANG = "C:/Users/hexay/acacia/acacia/.testserver/bds-blocks/resource_packs/vanilla/texts/en_US.lang"


def main():
    rows = []
    with open(LANG, encoding="utf-8-sig") as lang:
        for line in lang:
            key, _, text = line.rstrip("\r\n").partition("=")
            text = text.split("\t#", 1)[0].rstrip()
            if key.startswith("commands.") and text:
                rows.append(f"{key}\t{text}\n")
    with open(os.path.join(HERE, "..", "command-texts.tsv"), "w", encoding="utf-8", newline="\n") as out:
        out.writelines(rows)


if __name__ == "__main__":
    main()
