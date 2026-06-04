/* Copyright (C) 2000-2012 by George Williams */
/*
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:

 * Redistributions of source code must retain the above copyright notice, this
 * list of conditions and the following disclaimer.

 * Redistributions in binary form must reproduce the above copyright notice,
 * this list of conditions and the following disclaimer in the documentation
 * and/or other materials provided with the distribution.

 * The name of the author may not be used to endorse or promote products
 * derived from this software without specific prior written permission.

 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR IMPLIED
 * WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
 * EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#include <fontforge-config.h>

#include "encoding.h"
#include "ffglib_compat.h"
#include "fontforge.h"
#include "gfile.h"
#include "groups.h"
#include "macenc.h"
#include "namelist.h"
#include "othersubrs.h"
#include "sfd.h"
#include "splineutil.h"
#include "ttf.h"
#include "ustring.h"

#include <assert.h>
#include <locale.h>
#include <stdlib.h>
#ifndef _MSC_VER
#include <sys/time.h>
#endif

/* Stub preferences for headless library builds.
 * The former noprefs.c had extensive UI-related preferences and scripting
 * dependencies. This minimal version satisfies the prefs_interface so
 * the library can function without a GUI. */

static void NOUI_SavePrefs(int UNUSED(automatic)) {
    /* No-op in headless builds */
}

static void NOUI_LoadPrefs(void) {
    /* No-op in headless builds */
}

static int NOUI_GetPrefs(char *UNUSED(name), Val *UNUSED(val)) {
    return false;
}

static int NOUI_SetPrefs(char *UNUSED(name), Val *UNUSED(val1), Val *UNUSED(val2)) {
    return false;
}

static const char *NOUI_getFontForgeShareDir(void) {
    return getShareDir();
}

static void NOUI_SetDefaults(void) {
    /* Minimal defaults for headless operation */
    default_encoding = &custom;
}

static struct prefs_interface prefsnoui = {
    NOUI_SavePrefs,
    NOUI_LoadPrefs,
    NOUI_GetPrefs,
    NOUI_SetPrefs,
    NOUI_getFontForgeShareDir,
    NOUI_SetDefaults
};

struct prefs_interface *prefs_interface = &prefsnoui;

void FF_SetPrefsInterface(struct prefs_interface *prefsi) {
    prefs_interface = prefsi;
}
