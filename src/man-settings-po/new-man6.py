# man-settings 1man6: Beta Updates (TEST builds). Adds three strings to every
# LANG.po and rewrites tx/LANG.txt in the new order (same as new-man5.py).
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import tool

TEST = 'This is a TEST version. It can have bugs.'
BETA = 'Beta Updates'
BETA_SUB = 'Get TEST versions of MAN before they are released. They can have bugs.'
T = {
'af': ('Dit is ’n TEST-weergawe. Dit kan foute hê.', 'Beta-bywerkings', 'Kry TEST-weergawes van MAN voordat hulle vrygestel word. Hulle kan foute hê.'),
'am': ('ይህ የTEST ስሪት ነው። ስህተቶች ሊኖሩት ይችላሉ።', 'የቤታ ዝማኔዎች', 'የMAN TEST ስሪቶችን ከመለቀቃቸው በፊት ያግኙ። ስህተቶች ሊኖሯቸው ይችላሉ።'),
'ar': ('هذا إصدار TEST. قد يحتوي على أخطاء.', 'تحديثات بيتا', 'احصل على إصدارات TEST من MAN قبل إصدارها. قد تحتوي على أخطاء.'),
'az': ('Bu TEST versiyasıdır. Xətaları ola bilər.', 'Beta yeniləmələri', 'MAN-ın TEST versiyalarını buraxılışdan əvvəl alın. Onların xətaları ola bilər.'),
'be': ('Гэта версія TEST. У ёй могуць быць памылкі.', 'Бэта-абнаўленні', 'Атрымлівайце версіі TEST сістэмы MAN да выпуску. У іх могуць быць памылкі.'),
'bg': ('Това е TEST версия. Може да има грешки.', 'Бета обновления', 'Получавайте TEST версии на MAN преди да бъдат издадени. Може да имат грешки.'),
'bn': ('এটি একটি TEST সংস্করণ। এতে ত্রুটি থাকতে পারে।', 'বেটা হালনাগাদ', 'প্রকাশের আগেই MAN-এর TEST সংস্করণ পান। এগুলিতে ত্রুটি থাকতে পারে।'),
'bs': ('Ovo je TEST verzija. Može imati greške.', 'Beta ažuriranja', 'Dobijte TEST verzije MAN-a prije nego što budu objavljene. Mogu imati greške.'),
'ca': ('Aquesta és una versió TEST. Pot tenir errors.', 'Actualitzacions beta', 'Obteniu versions TEST de MAN abans que es publiquin. Poden tenir errors.'),
'cs': ('Toto je verze TEST. Může obsahovat chyby.', 'Beta aktualizace', 'Získávejte verze TEST systému MAN ještě před vydáním. Mohou obsahovat chyby.'),
'cy': ('Fersiwn TEST yw hon. Gall fod ganddi wallau.', 'Diweddariadau beta', 'Cael fersiynau TEST o MAN cyn iddynt gael eu rhyddhau. Gall fod ganddynt wallau.'),
'da': ('Dette er en TEST-version. Den kan have fejl.', 'Betaopdateringer', 'Få TEST-versioner af MAN, før de udgives. De kan have fejl.'),
'de': ('Dies ist eine TEST-Version. Sie kann Fehler enthalten.', 'Beta-Aktualisierungen', 'TEST-Versionen von MAN vor ihrer Veröffentlichung erhalten. Sie können Fehler enthalten.'),
'el': ('Αυτή είναι έκδοση TEST. Μπορεί να έχει σφάλματα.', 'Ενημερώσεις beta', 'Λάβετε εκδόσεις TEST του MAN πριν κυκλοφορήσουν. Μπορεί να έχουν σφάλματα.'),
'eo': ('Ĉi tio estas TEST-versio. Ĝi povas havi erarojn.', 'Beta-ĝisdatigoj', 'Ricevu TEST-versiojn de MAN antaŭ ilia eldono. Ili povas havi erarojn.'),
'es': ('Esta es una versión TEST. Puede tener errores.', 'Actualizaciones beta', 'Obtenga versiones TEST de MAN antes de su lanzamiento. Pueden tener errores.'),
'et': ('See on TEST-versioon. Selles võib olla vigu.', 'Beetauuendused', 'Saa MAN-i TEST-versioonid enne nende väljaandmist. Neis võib olla vigu.'),
'eu': ('TEST bertsio bat da hau. Akatsak izan ditzake.', 'Beta eguneraketak', 'Jaso MANen TEST bertsioak argitaratu aurretik. Akatsak izan ditzakete.'),
'fa': ('این یک نسخهٔ TEST است. ممکن است اشکال داشته باشد.', 'به‌روزرسانی‌های بتا', 'نسخه‌های TEST از MAN را پیش از انتشار دریافت کنید. ممکن است اشکال داشته باشند.'),
'fil': ('Ito ay isang bersyong TEST. Maaaring may mga bug ito.', 'Mga beta na update', 'Kunin ang mga bersyong TEST ng MAN bago ilabas ang mga ito. Maaaring may mga bug ang mga ito.'),
'fi': ('Tämä on TEST-versio. Siinä voi olla virheitä.', 'Beetapäivitykset', 'Saat MANin TEST-versiot ennen niiden julkaisua. Niissä voi olla virheitä.'),
'fr': ('Ceci est une version TEST. Elle peut contenir des bogues.', 'Mises à jour bêta', 'Recevoir les versions TEST de MAN avant leur sortie. Elles peuvent contenir des bogues.'),
'ga': ('Is leagan TEST é seo. D’fhéadfadh fabhtanna a bheith ann.', 'Nuashonruithe béite', 'Faigh leaganacha TEST de MAN sula n-eisítear iad. D’fhéadfadh fabhtanna a bheith iontu.'),
'gl': ('Esta é unha versión TEST. Pode ter erros.', 'Actualizacións beta', 'Obteña versións TEST de MAN antes de que se publiquen. Poden ter erros.'),
'gu': ('આ TEST આવૃત્તિ છે. તેમાં ભૂલો હોઈ શકે છે.', 'બીટા અપડેટ', 'MANની TEST આવૃત્તિઓ રજૂ થાય તે પહેલાં મેળવો. તેમાં ભૂલો હોઈ શકે છે.'),
'he': ('זוהי גרסת TEST. ייתכנו בה באגים.', 'עדכוני בטא', 'קבלת גרסאות TEST של MAN לפני שהן יוצאות. ייתכנו בהן באגים.'),
'hi': ('यह एक TEST संस्करण है। इसमें त्रुटियाँ हो सकती हैं।', 'बीटा अद्यतन', 'MAN के TEST संस्करण जारी होने से पहले पाएँ। इनमें त्रुटियाँ हो सकती हैं।'),
'hr': ('Ovo je TEST inačica. Može imati pogreške.', 'Beta ažuriranja', 'Primajte TEST inačice MAN-a prije njihova objavljivanja. Mogu imati pogreške.'),
'hu': ('Ez egy TEST verzió. Hibák lehetnek benne.', 'Béta frissítések', 'A MAN TEST verzióinak megkapása a kiadásuk előtt. Hibák lehetnek bennük.'),
'hy': ('Սա TEST տարբերակ է։ Այն կարող է սխալներ ունենալ։', 'Բետա թարմացումներ', 'Ստացեք MAN-ի TEST տարբերակները դրանց թողարկումից առաջ։ Դրանք կարող են սխալներ ունենալ։'),
'id': ('Ini adalah versi TEST. Mungkin ada bug.', 'Pembaruan beta', 'Dapatkan versi TEST MAN sebelum dirilis. Mungkin ada bug.'),
'is': ('Þetta er TEST-útgáfa. Hún getur verið með villur.', 'Beta-uppfærslur', 'Fáðu TEST-útgáfur af MAN áður en þær koma út. Þær geta verið með villur.'),
'it': ('Questa è una versione TEST. Può contenere errori.', 'Aggiornamenti beta', 'Ricevi le versioni TEST di MAN prima del rilascio. Possono contenere errori.'),
'ja': ('これは TEST 版です。不具合がある可能性があります。', 'ベータ版のアップデート', 'リリース前の MAN の TEST 版を受け取ります。不具合がある可能性があります。'),
'ka': ('ეს TEST ვერსიაა. შეიძლება ჰქონდეს შეცდომები.', 'ბეტა განახლებები', 'მიიღეთ MAN-ის TEST ვერსიები გამოშვებამდე. მათ შეიძლება ჰქონდეთ შეცდომები.'),
'kk': ('Бұл – TEST нұсқасы. Онда қателер болуы мүмкін.', 'Бета жаңартулар', 'MAN-ның TEST нұсқаларын шыққанға дейін алыңыз. Оларда қателер болуы мүмкін.'),
'kn': ('ಇದು TEST ಆವೃತ್ತಿ. ಇದರಲ್ಲಿ ದೋಷಗಳಿರಬಹುದು.', 'ಬೀಟಾ ನವೀಕರಣಗಳು', 'MAN ನ TEST ಆವೃತ್ತಿಗಳನ್ನು ಬಿಡುಗಡೆಗೆ ಮೊದಲೇ ಪಡೆಯಿರಿ. ಇವುಗಳಲ್ಲಿ ದೋಷಗಳಿರಬಹುದು.'),
'ko': ('TEST 버전입니다. 버그가 있을 수 있습니다.', '베타 업데이트', '출시 전에 MAN의 TEST 버전을 받습니다. 버그가 있을 수 있습니다.'),
'lt': ('Tai TEST versija. Joje gali būti klaidų.', 'Beta naujinimai', 'Gaukite MAN TEST versijas prieš jas išleidžiant. Jose gali būti klaidų.'),
'lv': ('Šī ir TEST versija. Tajā var būt kļūdas.', 'Beta atjauninājumi', 'Saņemiet MAN TEST versijas pirms to izlaišanas. Tajās var būt kļūdas.'),
'mk': ('Ова е TEST верзија. Може да има грешки.', 'Бета ажурирања', 'Добивајте TEST верзии на MAN пред да бидат објавени. Може да имаат грешки.'),
'ml': ('ഇതൊരു TEST പതിപ്പാണ്. ഇതിൽ പിശകുകൾ ഉണ്ടാകാം.', 'ബീറ്റ അപ്‌ഡേറ്റുകൾ', 'MAN-ന്റെ TEST പതിപ്പുകൾ പുറത്തിറങ്ങുന്നതിന് മുമ്പ് നേടുക. അവയിൽ പിശകുകൾ ഉണ്ടാകാം.'),
'mn': ('Энэ бол TEST хувилбар. Алдаа байж болно.', 'Бета шинэчлэлт', 'MAN-ын TEST хувилбаруудыг гарахаас өмнө аваарай. Алдаа байж болно.'),
'mr': ('ही TEST आवृत्ती आहे. यात त्रुटी असू शकतात.', 'बीटा अद्यतने', 'MAN च्या TEST आवृत्त्या प्रकाशित होण्यापूर्वी मिळवा. त्यांत त्रुटी असू शकतात.'),
'ms': ('Ini ialah versi TEST. Ia mungkin mempunyai pepijat.', 'Kemas kini beta', 'Dapatkan versi TEST MAN sebelum ia dikeluarkan. Ia mungkin mempunyai pepijat.'),
'nb': ('Dette er en TEST-versjon. Den kan ha feil.', 'Betaoppdateringer', 'Få TEST-versjoner av MAN før de blir utgitt. De kan ha feil.'),
'ne': ('यो TEST संस्करण हो। यसमा त्रुटिहरू हुन सक्छन्।', 'बिटा अद्यावधिकहरू', 'MAN का TEST संस्करणहरू सार्वजनिक हुनुअघि नै पाउनुहोस्। तिनमा त्रुटिहरू हुन सक्छन्।'),
'nl': ('Dit is een TEST-versie. Er kunnen fouten in zitten.', 'Bèta-updates', 'Ontvang TEST-versies van MAN voordat ze uitkomen. Er kunnen fouten in zitten.'),
'pa': ('ਇਹ TEST ਵਰਜਨ ਹੈ। ਇਸ ਵਿੱਚ ਗਲਤੀਆਂ ਹੋ ਸਕਦੀਆਂ ਹਨ।', 'ਬੀਟਾ ਅੱਪਡੇਟ', 'MAN ਦੇ TEST ਵਰਜਨ ਜਾਰੀ ਹੋਣ ਤੋਂ ਪਹਿਲਾਂ ਪ੍ਰਾਪਤ ਕਰੋ। ਇਹਨਾਂ ਵਿੱਚ ਗਲਤੀਆਂ ਹੋ ਸਕਦੀਆਂ ਹਨ।'),
'pl': ('To jest wersja TEST. Może zawierać błędy.', 'Aktualizacje beta', 'Otrzymuj wersje TEST systemu MAN przed ich wydaniem. Mogą zawierać błędy.'),
'pt_BR': ('Esta é uma versão TEST. Ela pode ter erros.', 'Atualizações beta', 'Receba versões TEST do MAN antes do lançamento. Elas podem ter erros.'),
'pt': ('Esta é uma versão TEST. Pode ter erros.', 'Atualizações beta', 'Receba versões TEST do MAN antes de serem lançadas. Podem ter erros.'),
'ro': ('Aceasta este o versiune TEST. Poate avea erori.', 'Actualizări beta', 'Primiți versiuni TEST ale MAN înainte de lansare. Pot avea erori.'),
'ru': ('Это версия TEST. В ней могут быть ошибки.', 'Бета-обновления', 'Получать версии TEST системы MAN до их выпуска. В них могут быть ошибки.'),
'si': ('මෙය TEST අනුවාදයකි. එහි දෝෂ තිබිය හැක.', 'බීටා යාවත්කාලීන', 'MAN හි TEST අනුවාද නිකුත් කිරීමට පෙර ලබා ගන්න. ඒවායේ දෝෂ තිබිය හැක.'),
'sk': ('Toto je verzia TEST. Môže obsahovať chyby.', 'Beta aktualizácie', 'Získavajte verzie TEST systému MAN ešte pred vydaním. Môžu obsahovať chyby.'),
'sl': ('To je različica TEST. Lahko vsebuje napake.', 'Beta posodobitve', 'Prejmite različice TEST sistema MAN, preden so izdane. Lahko vsebujejo napake.'),
'sq': ('Ky është një version TEST. Mund të ketë gabime.', 'Përditësime beta', 'Merrni versionet TEST të MAN përpara se të dalin. Mund të kenë gabime.'),
'sr': ('Ово је TEST издање. Може имати грешке.', 'Бета ажурирања', 'Добијајте TEST издања MAN-а пре њиховог објављивања. Могу имати грешке.'),
'sv': ('Det här är en TEST-version. Den kan ha fel.', 'Betauppdateringar', 'Få TEST-versioner av MAN innan de släpps. De kan ha fel.'),
'sw': ('Hili ni toleo la TEST. Linaweza kuwa na hitilafu.', 'Masasisho ya beta', 'Pata matoleo ya TEST ya MAN kabla hayajatolewa. Yanaweza kuwa na hitilafu.'),
'ta': ('இது TEST பதிப்பு. இதில் பிழைகள் இருக்கலாம்.', 'பீட்டா புதுப்பிப்புகள்', 'MAN இன் TEST பதிப்புகளை வெளியீட்டுக்கு முன்பே பெறுக. அவற்றில் பிழைகள் இருக்கலாம்.'),
'te': ('ఇది TEST సంస్కరణ. ఇందులో లోపాలు ఉండవచ్చు.', 'బీటా నవీకరణలు', 'MAN యొక్క TEST సంస్కరణలను విడుదలకు ముందే పొందండి. వాటిలో లోపాలు ఉండవచ్చు.'),
'th': ('นี่คือรุ่น TEST อาจมีข้อบกพร่อง', 'อัปเดตรุ่นเบต้า', 'รับ MAN รุ่น TEST ก่อนเปิดตัว ซึ่งอาจมีข้อบกพร่อง'),
'tr': ('Bu bir TEST sürümüdür. Hatalar içerebilir.', 'Beta güncellemeleri', 'MAN’in TEST sürümlerini yayımlanmadan önce alın. Hatalar içerebilirler.'),
'uk': ('Це версія TEST. У ній можуть бути помилки.', 'Бета-оновлення', 'Отримувати версії TEST системи MAN до їхнього випуску. У них можуть бути помилки.'),
'ur': ('یہ TEST ورژن ہے۔ اس میں خرابیاں ہو سکتی ہیں۔', 'بیٹا اپ ڈیٹس', 'MAN کے TEST ورژن جاری ہونے سے پہلے حاصل کریں۔ ان میں خرابیاں ہو سکتی ہیں۔'),
'uz': ('Bu TEST versiyasi. Unda xatolar bo‘lishi mumkin.', 'Beta yangilanishlar', 'MAN’ning TEST versiyalarini chiqishidan oldin oling. Ularda xatolar bo‘lishi mumkin.'),
'vi': ('Đây là phiên bản TEST. Có thể có lỗi.', 'Bản cập nhật beta', 'Nhận các phiên bản TEST của MAN trước khi phát hành. Chúng có thể có lỗi.'),
'zh_CN': ('这是 TEST 版本，可能存在错误。', 'Beta 更新', '在正式发布前获取 MAN 的 TEST 版本。这些版本可能存在错误。'),
'zh_TW': ('這是 TEST 版本，可能存在錯誤。', 'Beta 更新', '在正式發布前取得 MAN 的 TEST 版本。這些版本可能存在錯誤。'),
}

if __name__ == '__main__':
    messages = tool.msgs()
    ids = list(messages)
    for po in sorted(tool.HERE.glob('*.po')):
        lang = po.stem
        tr = tool.read_po(po)
        tr[TEST], tr[BETA], tr[BETA_SUB] = T[lang]
        missing = [m for m in ids if m not in tr]
        if missing:
            sys.exit(f'{lang}: missing {missing}')
        tool.write_po(po, messages, tr, lang)
        lines = [f'{i}|' + tr[m].replace('\n', '\\n') for i, m in enumerate(ids)]
        (tool.HERE / 'tx' / f'{lang}.txt').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(len(T), 'languages,', len(ids), 'messages')
