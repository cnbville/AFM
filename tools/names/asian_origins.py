"""Split the Census surname file's Asian and Pacific Islander carriers by
where the family comes from (faces item F4: East, Southeast and South
Asian players get the names those families carry).

Writes data/names/asian_surnames.csv: name, origin, people. `people` is
the name's count times its Asian/Pacific Islander share, times the origin's
part when two origins share the name (Lee: Korean, Chinese and Hmong).

How a name gets its origin, in order:
1. Hand lists of common names per origin (Census counts, the 3D faces
   thread's asian_names.py, extended here), and shared names with rough
   splits (estimates).
2. Chamorro (Guam) names and names only women carry (Kaur, Begum) are left
   out: they are not an Asian man's surname.
3. A name with 15% or more Hispanic carriers is Filipino (Aquino, Pascual).
4. A name whose letters read as Japanese syllables (Nakashima, Inoue).
5. One syllable (Chen, Wong, Huang) is Chinese.
6. Indic and Arabic letter patterns (Mishra, Srivastava, Banerjee, Ashraf)
   are South Asian; Tagalog ones (Pangilinan, Cunanan) Filipino.
7. Anything else is left out rather than guessed.
Names below 25% Asian/Pacific Islander carriers (Santos, Young) are
general American names: Cantonese names spelled as English words (Gee,
Chinn, Law) are Chinese, Spanish ones Filipino, and the rest go 70%
Filipino, 20% South Asian (Thomas, George: South Indian Christian
families) and 10% Chinese; names under 2% (Smith) are left out.

Usage: python3 -I asian_origins.py data/names/surnames.csv data/names/asian_surnames.csv
"""

import csv
import re
import sys

W = str.split
ORIGIN = {
    "korean": W(
        "KIM PARK CHOI CHUNG CHO KANG HAN SONG SHIN YI HWANG OH YOON JUNG KO CHUN YOO KWON PAK AHN CHOE YUN SON JANG SUH SEO IM "
        "CHON SHIM KOO RHEE WON KWAK JUN JEONG BANG PAIK JO JEON JOO CHAE BAEK HUH BYUN HYUN RO JU MUN HUR MOON HAM HAHN SOHN BAE "
        "YOU RYU NOH YOUN UM PAEK WHANG PYO SEOL CHI CHA NA RA YANG MYUNG CHOUNG"
    ),
    "japanese": W(
        "NAKAMURA TANAKA YAMAMOTO SATO WATANABE SUZUKI TAKAHASHI ITO MATSUMOTO YAMADA YOSHIDA SAITO KOBAYASHI KATO KIMURA HIGA "
        "OSHIRO SASAKI HAYASHI YAMAGUCHI MURAKAMI SAKAMOTO HASHIMOTO SHIMIZU FUJIMOTO IKEDA INOUYE NISHIMURA NAKAGAWA YAMASHITA "
        "KANESHIRO ABE MORI FUJII OKAMOTO SAKAI ONO MIYAMOTO NAKANO MATSUDA MAEDA OGAWA OKADA OTA HARADA HONDA TAMURA ISHII "
        "MORITA YAMASAKI AOKI FUJITA HASEGAWA ISHIKAWA ARAKAKI MIYASHIRO MASUDA GOTO ENDO KONDO ANDO KUDO SHIRAI"
    ),
    "vietnamese": W(
        "NGUYEN TRAN LE PHAM HUYNH TRUONG PHAN VU VO NGO DANG HOANG DO BUI DUONG LUU DINH TRINH LUONG MAI DOAN DAO QUACH VUONG TA "
        "PHUNG DIEP TRIEU LIEU TIEU BANH GIANG DAM TRANG PHU TIEN TON VAN KHUU LUC MACH THAN TAT KIEU NGHIEM HUA LAM"
    ),
    "hmong": W("VANG XIONG THAO HER MOUA VUE LOR HANG KHANG THOR KUE YANG LEE CHANG VUE"),
    "mien": W("SAECHAO SAELEE SAEPHAN SAETERN SAEYANG SAETEURN"),
    "cambodian": W("SOK CHEA KEO TAING MEAS SOM KHIEU PRAK CHHAY SOUN OUK YIM PEN NOU NUON"),
    "filipino": W(
        "DEGUZMAN DOMINGO TOLENTINO DELROSARIO DIZON CORPUZ IGNACIO PASCUA DEVERA ESPIRITU ASUNCION AGUSTIN ANCHETA DELOSREYES "
        "SISON MANALO LEGASPI GALANG VIRAY UY SY MALLARI PANGILINAN CUNANAN MACARAEG DUMLAO TUAZON BACANI DIMACULANGAN SALVADOR "
        "MANALASTAS MAGBANUA DAGDAG LUMBA BALUYUT DATU CABRAL"
    ),
    "south_asian": W(
        "PATEL SINGH SHAH KHAN AHMED SHARMA KUMAR DESAI GUPTA MEHTA HUSSAIN RAHMAN MATHEW RAO AHMAD REDDY MALIK JAIN AMIN JOSHI "
        "PRASAD CHOWDHURY PARIKH ISLAM SYED VARGHESE SIDDIQUI GANDHI SANDHU QURESHI BHATT BHAKTA ALAM DAS CHAUDHRY NAIR RANA "
        "IQBAL SHAIKH BHATIA DHILLON VERMA HOSSAIN SHETH SHEIKH DAVE GREWAL SIDHU MAHMOOD ARORA AGARWAL UDDIN TRIVEDI SRINIVASAN "
        "MISTRY LAL SINHA MIRZA PANDYA CHACKO PAREKH CHANDRA BHATTI KAPOOR KULKARNI KRISHNAN VYAS AKHTAR GHOSH IYER CHOUDHURY "
        "SAINI BAIG HAQUE SUBRAMANIAN RAI MALHOTRA ANAND BAINS NAIK SHUKLA AGRAWAL THAKKAR MODI KHANNA CHAND KAPADIA ZAMAN "
        "DHALIWAL CHOPRA PARMAR RIZVI SONI RAM HUSAIN SEN DSOUZA MENON DOSHI RAZA ALI MOHAMMED HASAN HASSAN RASHID KARIM ANSARI "
        "BUTT GILL ROY MOHAN PERSAUD PERERA JOSEPH ABRAHAM JOHN JACOB SAMUEL PHILIP PAL RAJ JAN RAJA SAHA BHAT GARG GOEL VORA "
        "AZIZ MOMIN DEY BOSE BRAR SETHI DALAL PATIL RAJAN KURIAN CHERIAN PILLAI MATHAI MISHRA SRIVASTAVA SAXENA SHRESTHA ABBAS "
        "HAIDER SALEEM ASLAM ANWAR ASHRAF KHALID ZAIDI REHMAN MOHAMMAD NARAYAN CHAUHAN BANERJEE MUKHERJEE CHAWLA CHEEMA AGGARWAL "
        "LAKHANI SULTANA RANDHAWA PRAKASH PATHAK KOTHARI SHANKAR VARUGHESE MATHUR"
    ),
}
SPLIT = {
    "LEE": {"korean": 0.45, "chinese": 0.45, "hmong": 0.10}, "YANG": {"chinese": 0.6, "hmong": 0.35, "korean": 0.05},
    "CHANG": {"chinese": 0.65, "korean": 0.2, "hmong": 0.15}, "LIM": {"korean": 0.4, "chinese": 0.45, "filipino": 0.15},
    "HO": {"chinese": 0.5, "vietnamese": 0.5}, "LAM": {"chinese": 0.6, "vietnamese": 0.4},
    "LY": {"vietnamese": 0.6, "chinese": 0.2, "cambodian": 0.2}, "LO": {"chinese": 0.6, "hmong": 0.4},
    "CHA": {"korean": 0.5, "hmong": 0.5}, "KONG": {"chinese": 0.6, "hmong": 0.4}, "TANG": {"chinese": 0.7, "vietnamese": 0.3},
    "TAN": {"chinese": 0.6, "filipino": 0.25, "cambodian": 0.15}, "LU": {"chinese": 0.85, "vietnamese": 0.15},
    "HA": {"vietnamese": 0.6, "korean": 0.4}, "TO": {"vietnamese": 0.6, "chinese": 0.4}, "LA": {"vietnamese": 0.7, "chinese": 0.3},
    "TU": {"chinese": 0.7, "vietnamese": 0.3}, "QUAN": {"vietnamese": 0.6, "chinese": 0.4}, "CAO": {"vietnamese": 0.7, "chinese": 0.3},
    "CHAU": {"vietnamese": 0.6, "chinese": 0.4}, "DONG": {"chinese": 0.6, "vietnamese": 0.3, "korean": 0.1},
    "TONG": {"chinese": 0.6, "vietnamese": 0.4}, "THAI": {"vietnamese": 0.8, "chinese": 0.2}, "THACH": {"vietnamese": 0.5, "cambodian": 0.5},
    "LIEN": {"vietnamese": 0.5, "chinese": 0.5}, "HUA": {"chinese": 0.5, "vietnamese": 0.5}, "NAM": {"korean": 0.7, "vietnamese": 0.3},
    "AN": {"korean": 0.6, "vietnamese": 0.4}, "MIN": {"korean": 0.8, "chinese": 0.2}, "SIN": {"korean": 0.5, "chinese": 0.5},
    "SIM": {"korean": 0.7, "chinese": 0.3}, "YIM": {"korean": 0.5, "chinese": 0.3, "cambodian": 0.2}, "WOO": {"korean": 0.6, "chinese": 0.4},
    "HONG": {"korean": 0.6, "chinese": 0.4}, "SUNG": {"korean": 0.5, "chinese": 0.5}, "JI": {"korean": 0.5, "chinese": 0.5},
    "KIM": {"korean": 0.98, "cambodian": 0.02}, "UNG": {"cambodian": 0.5, "chinese": 0.5}, "HENG": {"cambodian": 0.5, "chinese": 0.5},
    "MAO": {"chinese": 0.7, "cambodian": 0.3}, "SAM": {"cambodian": 0.5, "chinese": 0.5}, "SENG": {"cambodian": 0.6, "chinese": 0.4},
    "LAY": {"cambodian": 0.5, "chinese": 0.5}, "VONG": {"chinese": 0.5, "vietnamese": 0.5}, "GO": {"filipino": 0.6, "korean": 0.4},
    "YAP": {"filipino": 0.6, "chinese": 0.4}, "CHUA": {"filipino": 0.6, "chinese": 0.4}, "ONG": {"filipino": 0.4, "chinese": 0.6},
    "ANG": {"filipino": 0.4, "chinese": 0.6}, "FERNANDO": {"filipino": 0.6, "south_asian": 0.4}, "DAVID": {"filipino": 0.5, "south_asian": 0.5},
    "JOSE": {"filipino": 0.7, "south_asian": 0.3}, "PAI": {"south_asian": 0.6, "korean": 0.4},
    "RODRIGUES": {"filipino": 0.5, "south_asian": 0.5}, "SILVA": {"filipino": 0.6, "south_asian": 0.4},
    "CHI": {"korean": 0.5, "chinese": 0.5}, "CHE": {"chinese": 0.6, "korean": 0.4}, "CHOO": {"korean": 0.6, "chinese": 0.4},
    "YEO": {"korean": 0.5, "chinese": 0.5}, "DY": {"filipino": 0.7, "chinese": 0.3}, "CO": {"filipino": 0.7, "chinese": 0.3},
    "PEN": {"cambodian": 0.6, "chinese": 0.4}, "MAC": {"vietnamese": 0.8, "chinese": 0.2},
}
CHAMORRO = set(W("BORJA CAMACHO CRISOSTOMO MENDIOLA NATIVIDAD TAITANO BLAS QUINATA SABLAN TAIMANGLO"))
WOMEN = set(W("KAUR BEGUM KHATUN BIBI SULTANA"))
GROUP = {
    "chinese": "east", "korean": "east", "japanese": "east", "vietnamese": "southeast", "filipino": "southeast",
    "hmong": "southeast", "mien": "southeast", "cambodian": "southeast", "south_asian": "south",
}
# Census 2023 shares of Asian Americans by group (asian-faces.md, section 6).
GROUP_SHARE = {"east": 0.385, "southeast": 0.351, "south": 0.264}
LISTED = {n: o for o, names in ORIGIN.items() for n in names}

JP_SUFFIX = re.compile(
    "(moto|mura|kawa|gawa|yama|zaki|saki|hara|wara|naka|shima|jima|uchi|oka|tani|zawa|sawa|hashi|bayashi|ishi|kubo|"
    "mori|ta|da|no|to|do|be|ki|gi|ri|ue|zu|tsu|chi|shi|ji)$"
)
JP_ONSET = "(?:ky|sh|ch|ts|ry|ny|hy|my|gy|by|py|[kstnhmyrwgzdbpfj])"
JAPANESE = re.compile(f"^(?:{JP_ONSET}?[aeiou]|n(?=[^aeiouy]))+n?$")
# Pacific Islander names (the Census groups them with Asians): Samoan, Tongan and Hawaiian letters only, every consonant
# followed by a vowel, with an l, f, v or apostrophe.
PACIFIC = re.compile("^(?:[fghklmnpstvw']?[aeiou])+$")
CN_ONSET = "(?:zh|ch|sh|ts|hs|ng|[bpmfdtnlgkhjqxrzcsyw])"
CN_RIME = (
    "(?:a|ai|an|ang|ao|au|ah|e|ei|en|eng|eu|ee|eung|i|ia|ian|iang|iao|ie|ien|in|ing|iong|iu|o|oi|on|ong|oo|ou|ow|oy|"
    "u|ua|uan|uang|ue|ui|un|ung|uo|um|am|im|ip|ak|ok|uk|ik|at|ut|ew|ue|eh|oh|ih|uh)"
)
CHINESE = re.compile(f"^{CN_ONSET}?{CN_RIME}$")
SOUTH_ASIAN = re.compile(
    "(bh|dh|kh|gh|jh|aa|ee|ii)|(jee|kar|wal|wala|wani|ani|ia|ra|esh|ish|raj|pal|dra|nath|deep|appa|aiah|ullah|uddin|zad|vedi|"
    "rao|ar|an)$"
)
TAGALOG = re.compile("(^(mac|mag|man|pan|dum|bac|cab|dim|lac|tag|bun|cun|pang|ding|bat|mal|pag)|ng[aeiou]|ao|uan$)")


def origin_of(name, hispanic):
    key = name.upper().replace(" ", "").replace("'", "").replace("-", "")
    if key in CHAMORRO or key in WOMEN:
        return None
    if key in SPLIT:
        return SPLIT[key]
    if key in LISTED:
        return {LISTED[key]: 1.0}
    low = name.lower().replace(" ", "").replace("-", "")
    if hispanic >= 15:
        return {"filipino": 1.0}
    if PACIFIC.match(low) and re.search("[lfv']", low):
        return None
    # Japanese has no ti, tu, si, zi, hu, di, du, wi, we, yi, ye or je syllables (Tiwari is not Japanese).
    not_jp = re.search("[lvqxc]|t[iu]|[sz]i|hu|d[iu]|w[ie]|y[ie]|je", low.replace("chi", "").replace("tsu", "").replace("shi", ""))
    if len(low) >= 4 and JAPANESE.match(low) and JP_SUFFIX.search(low) and not not_jp:
        return {"japanese": 1.0}
    if CHINESE.match(low) or low == "ng":
        return {"chinese": 1.0}
    if TAGALOG.search(low):
        return {"filipino": 1.0}
    if SOUTH_ASIAN.search(low) and not PACIFIC.match(low):
        return {"south_asian": 1.0}
    return None


def main(src, dst):
    out = {}
    general = 0.0
    for r in csv.DictReader(open(src, newline="")):
        try:
            count, api, his = float(r["count"]), float(r["pct_api"] or 0), float(r["pct_hispanic"] or 0)
        except ValueError:
            continue
        people = count * api / 100
        if people < 1:
            continue
        name = r["name"]
        if api < 25:
            # A general American name: Spanish ones are Filipino families', the rest spread over the groups. Names
            # where Asian carriers are under 2% are noise for this purpose and left out.
            if api < 2:
                continue
            low = name.lower()
            if api >= 8 and CHINESE.match(low):
                parts = {"chinese": 1.0}  # Cantonese names spelled as English words: Gee, Chinn, Law, Young
            elif his >= 30:
                parts = {"filipino": 1.0}
            else:
                parts = {"filipino": 0.7, "south_asian": 0.2, "chinese": 0.1}
            general += people
        else:
            parts = origin_of(name, his)
            if parts is None:
                continue
        for o, f in parts.items():
            out[(name, o)] = out.get((name, o), 0.0) + people * f
    rows = sorted(out.items(), key=lambda kv: (-kv[1], kv[0]))
    with open(dst, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["name", "origin", "people"])
        for (name, o), p in rows:
            # Names with fewer than 10 such carriers are left out: 0.8% of the people, a third of the file.
            if p >= 10:
                w.writerow([name, o, round(p)])
    by = {}
    for (name, o), p in rows:
        by[GROUP[o]] = by.get(GROUP[o], 0) + p
    print("rows", len(rows), "general people", round(general), {k: round(v) for k, v in by.items()})


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
