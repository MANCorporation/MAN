/* pick.c: exercises every file dialog entry point, for testing MAN's native picker.
 *   pick open|openA|multi|save|saveA|itemopen|itemsave|itemfolder|folder
 * Build: x86_64-w64-mingw32-gcc / i686-w64-mingw32-gcc pick.c -o pick.exe -lcomdlg32 -lole32 -luuid -lshell32
 * The result is written to pick-result.txt in %TEMP%. */
#define COBJMACROS
#define _WIN32_WINNT 0x0600
#include <windows.h>
#include <commdlg.h>
#include <shobjidl.h>
#include <shlobj.h>
#include <stdio.h>
static FILE *out;
static void say(const WCHAR *what, const WCHAR *s) { fwprintf(out, L"%ls: %ls\n", what, s ? s : L"(none)"); fflush(out); }
int WINAPI wWinMain(HINSTANCE h, HINSTANCE p, WCHAR *cmd, int show)
{
    WCHAR path[MAX_PATH * 8] = L"", tmp[MAX_PATH];
    OPENFILENAMEW o = { sizeof(o) };
    GetTempPathW(MAX_PATH, tmp); lstrcatW(tmp, L"pick-result.txt");
    out = _wfopen(tmp, L"a");
    say(L"mode", cmd);
    o.lpstrFile = path; o.nMaxFile = ARRAYSIZE(path);
    o.lpstrFilter = L"Text files\0*.txt;*.log\0All files\0*.*\0"; o.nFilterIndex = 1; o.lpstrTitle = L"MAN test dialog"; o.lpstrDefExt = L"txt";
    if (!lstrcmpW(cmd, L"open") || !lstrcmpW(cmd, L"multi")) {
        o.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | (!lstrcmpW(cmd, L"multi") ? OFN_ALLOWMULTISELECT : 0);
        BOOL ok = GetOpenFileNameW(&o);
        fwprintf(out, L"ret %d err %lu offset %u ext %u\n", ok, CommDlgExtendedError(), o.nFileOffset, o.nFileExtension);
        if (ok) for (WCHAR *q = path; *q; q += lstrlenW(q) + 1) say(L"file", q);
    } else if (!lstrcmpW(cmd, L"save")) {
        o.Flags = OFN_EXPLORER | OFN_OVERWRITEPROMPT; lstrcpyW(path, L"new document");
        BOOL ok = GetSaveFileNameW(&o);
        fwprintf(out, L"ret %d err %lu offset %u ext %u\n", ok, CommDlgExtendedError(), o.nFileOffset, o.nFileExtension);
        if (ok) say(L"file", path);
    } else if (!lstrcmpW(cmd, L"openA") || !lstrcmpW(cmd, L"saveA")) {
        char a[MAX_PATH] = ""; OPENFILENAMEA oa = { sizeof(oa) };
        oa.lpstrFile = a; oa.nMaxFile = MAX_PATH; oa.lpstrFilter = "Text files\0*.txt\0All\0*.*\0"; oa.nFilterIndex = 1; oa.Flags = OFN_EXPLORER;
        BOOL ok = !lstrcmpW(cmd, L"openA") ? GetOpenFileNameA(&oa) : GetSaveFileNameA(&oa);
        fwprintf(out, L"ret %d err %lu offset %u ext %u\n%hs\n", ok, CommDlgExtendedError(), oa.nFileOffset, oa.nFileExtension, a);
    } else if (!lstrcmpW(cmd, L"folder")) {
        BROWSEINFOW b = {0}; b.lpszTitle = L"Choose a folder"; b.ulFlags = BIF_NEWDIALOGSTYLE; b.pszDisplayName = tmp;
        CoInitialize(NULL); PIDLIST_ABSOLUTE l = SHBrowseForFolderW(&b);
        if (l && SHGetPathFromIDListW(l, path)) say(L"folder", path); else say(L"folder", L"cancelled");
    } else {
        IFileDialog *d = NULL; HRESULT hr; DWORD f;
        CoInitialize(NULL);
        hr = CoCreateInstance(!lstrcmpW(cmd, L"itemsave") ? &CLSID_FileSaveDialog : &CLSID_FileOpenDialog, NULL, CLSCTX_INPROC_SERVER, &IID_IFileDialog, (void **)&d);
        fwprintf(out, L"create %08lx\n", hr);
        COMDLG_FILTERSPEC spec[2] = { { L"Text", L"*.txt" }, { L"All", L"*.*" } };
        IFileDialog_SetFileTypes(d, 2, spec); IFileDialog_SetTitle(d, L"MAN test item dialog"); IFileDialog_SetDefaultExtension(d, L"txt");
        IFileDialog_GetOptions(d, &f);
        if (!lstrcmpW(cmd, L"itemfolder")) IFileDialog_SetOptions(d, f | FOS_PICKFOLDERS);
        if (!lstrcmpW(cmd, L"itemsave")) IFileDialog_SetFileName(d, L"saved");
        hr = IFileDialog_Show(d, NULL);
        fwprintf(out, L"show %08lx\n", hr);
        if (SUCCEEDED(hr)) {
            IShellItem *i; WCHAR *n;
            hr = IFileDialog_GetResult(d, &i); fwprintf(out, L"result %08lx\n", hr);
            if (SUCCEEDED(hr)) { IShellItem_GetDisplayName(i, SIGDN_FILESYSPATH, &n); say(L"item", n); }
        }
    }
    fclose(out);
    return 0;
}
