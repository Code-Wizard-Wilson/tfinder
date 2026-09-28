"""Diagnostic probe of Laya question wording; performs no system actions."""
from __future__ import annotations

import json
from server import classify, get_agent

TEXTS = [
    "I would like to remove Google Chrome",
    "какая штука оккупировала 8765",
    "скачай Ubuntu ISO",
]

QUESTIONS = {
    "simple": {
        "type": "choice",
        "instructions": "What does the user want to do?",
        "criteria": {
            "remove_app": "uninstall or remove an installed application",
            "inspect_port": "find out which process is using a network port number",
            "other": "something else",
        },
    },
    "binary": {
        "type": "choice",
        "instructions": "Does the user ask to remove an installed application?",
        "criteria": {"yes": "remove, delete, or uninstall an app", "no": "another goal"},
    },
    "yesno": {
        "type": "noul",
        "instructions": "Does the user ask to remove an installed application?",
        "criteria": {"true": "asks to uninstall an app", "false": "does not ask to uninstall an app"},
    },
}

if __name__ == "__main__":
    agent = get_agent()
    for text in TEXTS:
        for state in [text, {"request": text}]:
            result = agent.predict(state, QUESTIONS)
            print(json.dumps({"state": state, "answers": result["answers"]}, ensure_ascii=False))
    directory_capabilities = [
        {"name": "DIRECTORY_SIZE", "description": "Show directory size", "examples": ["how large is Documents"]},
        {"name": "FIND_LARGE_FILES", "description": "Find largest files within a directory", "examples": ["show largest files in Downloads"]},
    ]
    for text in ["how large is Documents", "у меня Downloads занимает много места",
                 "show largest files in Downloads", "найди большие видео за месяц в Downloads",
                 "скачай Ubuntu ISO"]:
        result = classify({"text": text, "capabilities": directory_capabilities, "debug": True}, agent)
        print(json.dumps({"text": text, "result": result}, ensure_ascii=False))
    unsupported = {"goal": {"type": "choice", "instructions": "Infer the user's actual goal. Choose OTHER if it does not fit a listed goal.", "criteria": {
        "SET_DEFAULT_BROWSER": "Change which browser opens web links by default",
        "ENABLE_LOGIN_ITEM": "Make an application launch automatically when logging in",
        "DISABLE_LOGIN_ITEM": "Stop an application from launching automatically when logging in",
        "DOWNLOAD_FILE": "Download a file from the internet",
        "INSTALL_APP": "Install new software or an application",
        "UPDATE_OS": "Update macOS operating system",
        "SEND_EMAIL": "Send an email message to a person",
        "CREATE_ACCOUNT": "Create a new user account",
        "OTHER": "None of these goals describes the request",
    }}}
    for text in ["change default browser to Google Chrome", "enable Google Chrome at login",
                 "сделай Firefox браузером по умолчанию", "download latest Ubuntu ISO",
                 "поставь мне Docker", "обнови macOS", "запрети Discord запускаться при входе",
                 "send an email to Alex", "создай новый аккаунт", "у меня Downloads занимает много места"]:
        result = agent.predict(text, unsupported)
        print(json.dumps({"text": text, "unsupported": result.get("answers", {}).get("goal")}, ensure_ascii=False))
    for text in ["enable Google Chrome at login", "поставь мне Docker", "обнови macOS",
                 "запрети Discord запускаться при входе", "у меня Downloads занимает много места"]:
        question = {"goal": {"type": "choice", "instructions": "Choose the exact goal or OTHER.", "criteria": {
            "ENABLE_LOGIN_ITEM": "Enable an app to automatically open at login",
            "DISABLE_LOGIN_ITEM": "Prevent an app from automatically opening at login",
            "INSTALL_APP": "Install an application on this Mac",
            "UPDATE_OS": "Install macOS operating system updates",
            "OTHER": "A different goal, not one of those above",
        }}}
        result = agent.predict(text, question)
        print(json.dumps({"text": text, "shortlist": result.get("answers", {}).get("goal")}, ensure_ascii=False))
