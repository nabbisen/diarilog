# Project Fluent translation file (en)
#
# Translation guidelines:
# - Use natural, supportive language consistent with trauma-informed care
# - Avoid clinical jargon; aim for warm, inclusive tone
# - Crisis-related strings (`crisis-` prefix) require expert review

## ── Common ──
brand-name = Trauma Journal
back-to-top = Back to top
sign-in = Sign in
loading = Loading...

## ── Index page ──
index-title = Trauma Journal
index-tagline = A trauma-informed journaling platform
index-description = A private space to write at your own pace, for yourself.
index-skeleton-notice = This page is rendered with Leptos v0.8 SSR.

## ── Login page ──
login-title = Sign in
login-prompt = Continue to your OIDC provider to sign in.
login-issuer-label = issuer:
login-flow-not-implemented = (The redirect flow is not yet wired up in this skeleton.)
login-issuer-missing = OIDC provider is not configured. Please check the server settings.

## ── Dashboard page ──
dashboard-title = Dashboard
dashboard-greeting = Welcome, { $name }
dashboard-greeting-guest = Guest
dashboard-active-session-heading = You have an interview in progress.
dashboard-active-session-resume = Resume conversation
dashboard-recent-heading = Recent journals
dashboard-recent-empty = No journals yet.
dashboard-recent-fetch-failed = (We could not load recent journals right now. Please try reloading.)
dashboard-mood-label = mood:
dashboard-partial-degradation = Some data could not be loaded. Please try reloading later.
dashboard-unauthenticated = Could not load dashboard data. Please make sure you are signed in and reload.

## ── 404 ──
not-found-title = 404 - Not Found

## ── Settings page ──
settings-title = Settings
settings-profile-heading = Profile
settings-language-heading = Language
settings-danger-heading = Danger zone
settings-erase-heading = Erase all your data
settings-erase-description =
    This permanently deletes your profile, all diary entries, all interview
    sessions, and all settings — both on this device and on our servers.
    There is no recovery path.
settings-erase-suggestion = Consider exporting your data first.
settings-erase-export-link = Export my data
settings-erase-confirm-label = Type { $word } to confirm
settings-erase-confirm-word = ERASE
settings-erase-button = Erase everything
settings-erase-progress = Erasing…
settings-erase-done = Your data has been erased.

## ── Onboarding ──
onboarding-title = Welcome to diarilog
onboarding-intro =
    Before you start, we need to set up your encryption passphrase.
    This passphrase protects your journals so that only you can read them.
    We cannot reset it. If you lose it, your journals cannot be recovered.
onboarding-passphrase-label = Choose a passphrase
onboarding-passphrase-confirm-label = Re-enter your passphrase
onboarding-passphrase-hint =
    Use something memorable — a phrase, a combination of words.
    Write it down somewhere safe before continuing.
onboarding-passphrase-mismatch = Passphrases do not match.
onboarding-warning-label =
    I understand: if I lose my passphrase, my data cannot be recovered.
onboarding-continue = Set up and continue
onboarding-passphrase-strength-weak = Passphrase is too weak — please add more words or characters.
