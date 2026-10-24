# man-settings 1man5: Updates installs inside Settings. Adds one string to every
# LANG.po, drops the ones that are gone, and rewrites tx/LANG.txt in the new order.
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import tool

NEW = 'MAN %s Will Be Installed When You Restart'
T = {
'af': 'MAN %s word geïnstalleer wanneer jy herbegin',
'am': 'MAN %s እንደገና ሲያስጀምሩ ይጫናል',
'ar': 'سيُثبَّت MAN %s عند إعادة التشغيل',
'az': 'MAN %s yenidən başlatdığınızda quraşdırılacaq',
'be': 'MAN %s усталюецца пры перазапуску',
'bg': 'MAN %s ще се инсталира при рестартиране',
'bn': 'পুনরায় চালু করলে MAN %s ইনস্টল হবে',
'bs': 'MAN %s će se instalirati kada ponovo pokrenete računar',
'ca': 'MAN %s s’instal·larà quan reinicieu',
'cs': 'MAN %s se nainstaluje při restartu',
'cy': 'Bydd MAN %s yn cael ei osod pan fyddwch yn ailgychwyn',
'da': 'MAN %s installeres, når du genstarter',
'de': 'MAN %s wird beim Neustart installiert',
'el': 'Το MAN %s θα εγκατασταθεί κατά την επανεκκίνηση',
'eo': 'MAN %s instaliĝos kiam vi restartigos',
'es': 'MAN %s se instalará cuando reinicie',
'et': 'MAN %s paigaldatakse taaskäivitamisel',
'eu': 'MAN %s berrabiaraztean instalatuko da',
'fa': 'MAN %s هنگام راه‌اندازی دوباره نصب می‌شود',
'fil': 'Mai-install ang MAN %s kapag nag-restart ka',
'fi': 'MAN %s asennetaan, kun käynnistät uudelleen',
'fr': 'MAN %s sera installé au redémarrage',
'ga': 'Suiteálfar MAN %s nuair a atosaíonn tú',
'gl': 'MAN %s instalarase cando reinicie',
'gu': 'ફરી શરૂ કરશો ત્યારે MAN %s સ્થાપિત થશે',
'he': 'MAN %s תותקן בהפעלה מחדש',
'hi': 'पुनः आरंभ करने पर MAN %s स्थापित होगा',
'hr': 'MAN %s instalirat će se kad ponovno pokrenete računalo',
'hu': 'A MAN %s újraindításkor települ',
'hy': 'MAN %s-ը կտեղադրվի վերագործարկելիս',
'id': 'MAN %s akan dipasang saat Anda memulai ulang',
'is': 'MAN %s verður sett upp þegar þú endurræsir',
'it': 'MAN %s verrà installato al riavvio',
'ja': 'MAN %s は再起動時にインストールされます',
'ka': 'MAN %s დაინსტალირდება გადატვირთვისას',
'kk': 'MAN %s қайта іске қосқанда орнатылады',
'kn': 'ಮರುಪ್ರಾರಂಭಿಸಿದಾಗ MAN %s ಸ್ಥಾಪನೆಯಾಗುತ್ತದೆ',
'ko': 'MAN %s은(는) 다시 시작할 때 설치됩니다',
'lt': 'MAN %s bus įdiegta paleidus iš naujo',
'lv': 'MAN %s tiks instalēta, kad restartēsiet',
'mk': 'MAN %s ќе се инсталира кога ќе рестартирате',
'ml': 'പുനരാരംഭിക്കുമ്പോൾ MAN %s ഇൻസ്റ്റാൾ ചെയ്യും',
'mn': 'MAN %s дахин эхлүүлэхэд суулгагдана',
'mr': 'पुन्हा सुरू केल्यावर MAN %s स्थापित होईल',
'ms': 'MAN %s akan dipasang apabila anda memulakan semula',
'nb': 'MAN %s installeres når du starter på nytt',
'ne': 'पुनः सुरु गर्दा MAN %s स्थापना हुनेछ',
'nl': 'MAN %s wordt geïnstalleerd als je herstart',
'pa': 'ਮੁੜ-ਚਾਲੂ ਕਰਨ ’ਤੇ MAN %s ਸਥਾਪਤ ਹੋਵੇਗਾ',
'pl': 'MAN %s zostanie zainstalowany po ponownym uruchomieniu',
'pt_BR': 'O MAN %s será instalado quando você reiniciar',
'pt': 'O MAN %s será instalado quando reiniciar',
'ro': 'MAN %s va fi instalat la repornire',
'ru': 'MAN %s установится при перезапуске',
'si': 'නැවත ආරම්භ කළ විට MAN %s ස්ථාපනය වේ',
'sk': 'MAN %s sa nainštaluje pri reštarte',
'sl': 'MAN %s bo nameščen ob vnovičnem zagonu',
'sq': 'MAN %s do të instalohet kur të rinisni',
'sr': 'MAN %s ће се инсталирати када поново покренете рачунар',
'sv': 'MAN %s installeras när du startar om',
'sw': 'MAN %s itasakinishwa utakapowasha upya',
'ta': 'மறுதொடக்கம் செய்யும்போது MAN %s நிறுவப்படும்',
'te': 'పునఃప్రారంభించినప్పుడు MAN %s స్థాపించబడుతుంది',
'th': 'MAN %s จะถูกติดตั้งเมื่อคุณเริ่มระบบใหม่',
'tr': 'MAN %s yeniden başlattığınızda kurulacak',
'uk': 'MAN %s встановиться під час перезапуску',
'ur': 'دوبارہ شروع کرنے پر MAN %s انسٹال ہو گا',
'uz': 'MAN %s qayta ishga tushirganingizda o‘rnatiladi',
'vi': 'MAN %s sẽ được cài đặt khi bạn khởi động lại',
'zh_CN': 'MAN %s 将在重新启动时安装',
'zh_TW': 'MAN %s 將在重新啟動時安裝',
}

if __name__ == '__main__':
    messages = tool.msgs()
    ids = list(messages)
    for po in sorted(tool.HERE.glob('*.po')):
        lang = po.stem
        tr = tool.read_po(po)
        tr[NEW] = T[lang]
        missing = [m for m in ids if m not in tr]
        if missing:
            sys.exit(f'{lang}: missing {missing}')
        tool.write_po(po, messages, tr, lang)
        lines = [f'{i}|' + tr[m].replace('\n', '\\n') for i, m in enumerate(ids)]
        (tool.HERE / 'tx' / f'{lang}.txt').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(len(T), 'languages,', len(ids), 'messages')
