# man-settings 1man10: Login Screen group in Appearance (MAN Login). One new
# string; the other three ("Login Screen", "Background", "_Add Picture…") are
# Settings' own and are taken from its catalogs (gcc-51/po). Rewrites tx/LANG.txt
# in the new order (same as new-man6.py).
import re
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import tool

SAME = 'Same as Desktop Background'
FROM_SETTINGS = ['Login Screen', 'Background', '_Add Picture…']
T = {
'af': 'Dieselfde as werkskermagtergrond',
'am': 'ከዴስክቶፕ ዳራ ጋር ተመሳሳይ',
'ar': 'مثل خلفية سطح المكتب',
'az': 'İş masası fonu ilə eyni',
'be': 'Як фон працоўнага стала',
'bg': 'Като фона на работния плот',
'bn': 'ডেস্কটপের পটভূমির মতো',
'bs': 'Isto kao pozadina radne površine',
'ca': 'Igual que el fons de l’escriptori',
'cs': 'Stejné jako pozadí plochy',
'cy': 'Yr un fath â chefndir y bwrdd gwaith',
'da': 'Samme som skrivebordsbaggrunden',
'de': 'Wie der Schreibtischhintergrund',
'el': 'Ίδιο με το παρασκήνιο της επιφάνειας εργασίας',
'eo': 'Same kiel la labortabla fono',
'es': 'Igual que el fondo del escritorio',
'et': 'Sama mis töölaua taust',
'eu': 'Mahaigaineko atzeko planoaren berdina',
'fa': 'مانند پس‌زمینهٔ میزکار',
'fi': 'Sama kuin työpöydän tausta',
'fil': 'Kapareho ng background ng desktop',
'fr': 'Identique à l’arrière-plan du bureau',
'ga': 'Mar an gcéanna le cúlra na deisce',
'gl': 'Igual que o fondo do escritorio',
'gu': 'ડેસ્કટોપ પૃષ્ઠભૂમિ જેવું જ',
'he': 'כמו רקע שולחן העבודה',
'hi': 'डेस्कटॉप पृष्ठभूमि जैसा',
'hr': 'Isto kao pozadina radne površine',
'hu': 'Ugyanaz, mint az asztalháttér',
'hy': 'Նույնը, ինչ աշխատասեղանի ֆոնը',
'id': 'Sama seperti latar belakang desktop',
'is': 'Sama og bakgrunnur skjáborðs',
'it': 'Come lo sfondo della scrivania',
'ja': 'デスクトップの背景と同じ',
'ka': 'სამუშაო მაგიდის ფონის მსგავსად',
'kk': 'Жұмыс үстелінің фонымен бірдей',
'kn': 'ಡೆಸ್ಕ್‌ಟಾಪ್ ಹಿನ್ನೆಲೆಯಂತೆಯೇ',
'ko': '바탕 화면 배경과 같게',
'lt': 'Toks pat kaip darbalaukio fonas',
'lv': 'Tāds pats kā darbvirsmas fons',
'mk': 'Исто како позадината на работната површина',
'ml': 'ഡെസ്ക്ടോപ്പ് പശ്ചാത്തലം പോലെ തന്നെ',
'mn': 'Ширээний дэвсгэртэй адил',
'mr': 'डेस्कटॉप पार्श्वभूमीसारखीच',
'ms': 'Sama seperti latar belakang desktop',
'nb': 'Samme som skrivebordsbakgrunnen',
'ne': 'डेस्कटप पृष्ठभूमि जस्तै',
'nl': 'Gelijk aan de bureaubladachtergrond',
'pa': 'ਡੈਸਕਟਾਪ ਬੈਕਗ੍ਰਾਊਂਡ ਵਰਗਾ ਹੀ',
'pl': 'Taki sam jak tło pulpitu',
'pt': 'Igual ao fundo do ambiente de trabalho',
'pt_BR': 'Igual ao plano de fundo da área de trabalho',
'ro': 'La fel ca fundalul desktopului',
'ru': 'Как фон рабочего стола',
'si': 'ඩෙස්ක්ටොප් පසුබිම ලෙසම',
'sk': 'Rovnaké ako pozadie pracovnej plochy',
'sl': 'Enako kot ozadje namizja',
'sq': 'Njësoj si sfondi i desktopit',
'sr': 'Исто као позадина радне површи',
'sv': 'Samma som skrivbordsbakgrunden',
'sw': 'Sawa na mandharinyuma ya eneo-kazi',
'ta': 'பணிமேடை பின்னணியைப் போலவே',
'te': 'డెస్క్‌టాప్ నేపథ్యం లాగానే',
'th': 'เหมือนกับพื้นหลังเดสก์ท็อป',
'tr': 'Masaüstü arka planıyla aynı',
'uk': 'Як тло стільниці',
'ur': 'ڈیسک ٹاپ کے پس منظر جیسا',
'uz': 'Ish stoli foni bilan bir xil',
'vi': 'Giống hình nền màn hình nền',
'zh_CN': '与桌面背景相同',
'zh_TW': '與桌面背景相同',
}

# Where Settings has no translation of its own: (Login Screen, Background, _Add Picture…)
OWN = {
'af': ('Aanmeldskerm', None, 'Voeg prent _by…'),
'am': ('የመግቢያ ገጽ', 'ዳራ', 'ሥዕል _አክል…'),
'ar': ('شاشة الولوج', None, '_أضف صورة…'),
'az': ('Giriş ekranı', 'Fon', 'Şəkil ə_lavə et…'),
'bn': ('লগইন পর্দা', None, 'ছবি যোগ করুন… (_A)'),
'bs': (None, None, '_Dodaj sliku…'),
'cy': ('Sgrin mewngofnodi', 'Cefndir', '_Ychwanegu llun…'),
'et': (None, None, '_Lisa pilt…'),
'fil': ('Screen sa pag-login', 'Background', '_Magdagdag ng larawan…'),
'ga': (None, None, '_Cuir pictiúr leis…'),
'gu': (None, None, 'ચિત્ર ઉમેરો… (_A)'),
'hr': (None, None, '_Dodaj sliku…'),
'hy': ('Մուտքի էկրան', 'Ֆոն', '_Ավելացնել նկար…'),
'is': (None, None, '_Bæta við mynd…'),
'kn': (None, None, 'ಚಿತ್ರವನ್ನು ಸೇರಿಸಿ… (_A)'),
'mk': ('Екран за најава', None, '_Додај слика…'),
'ml': ('ലോഗിൻ സ്ക്രീൻ', 'പശ്ചാത്തലം', 'ചിത്രം ചേർക്കുക… (_A)'),
'mn': ('Нэвтрэх дэлгэц', 'Дэвсгэр', 'Зураг _нэмэх…'),
'mr': (None, None, 'चित्र जोडा… (_A)'),
'ms': ('Skrin log masuk', None, '_Tambah gambar…'),
'si': ('පිවිසුම් තිරය', None, 'පින්තූරයක් එක් කරන්න… (_A)'),
'sq': ('Ekrani i hyrjes', None, '_Shto figurë…'),
'sw': ('Skrini ya kuingia', 'Mandharinyuma', '_Ongeza picha…'),
'ta': (None, None, 'படத்தைச் சேர்… (_A)'),
'te': (None, None, 'చిత్రాన్ని జోడించు… (_A)'),
'ur': ('لاگ اِن اسکرین', 'پس منظر', 'تصویر شامل کریں… (_A)'),
}


def settings_translation(lang, msgid):
    """Settings' own translation of msgid, or None."""
    po = tool.GCC / 'po' / f'{lang}.po'
    if not po.exists():
        return None
    text = po.read_text(encoding='utf-8')
    quoted = tool.po_quote(msgid)
    m = re.search(r'^msgid ' + re.escape(quoted) + r'\nmsgstr ((?:"(?:[^"\\]|\\.)*"\n?)+)', text, re.M)
    if not m:
        return None
    s = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1)))
    s = s.encode('utf-8').decode('unicode_escape').encode('latin-1').decode('utf-8') if '\\' in s else s
    return s or None


if __name__ == '__main__':
    messages = tool.msgs()
    ids = list(messages)
    for po in sorted(tool.HERE.glob('*.po')):
        lang = po.stem
        tr = tool.read_po(po)
        tr[SAME] = T[lang]
        for i, msgid in enumerate(FROM_SETTINGS):
            own = OWN.get(lang, (None, None, None))[i]
            tr[msgid] = settings_translation(lang, msgid) or own or msgid
        missing = [m for m in ids if m not in tr]
        if missing:
            sys.exit(f'{lang}: missing {missing}')
        tool.write_po(po, messages, tr, lang)
        lines = [f'{i}|' + tr[m].replace('\n', '\\n') for i, m in enumerate(ids)]
        (tool.HERE / 'tx' / f'{lang}.txt').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(len(T), 'languages,', len(ids), 'messages')
