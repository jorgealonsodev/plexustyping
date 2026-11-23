//! Every backend message the interface shows verbatim, in the five interface
//! languages (ui-i18n S1). The Spanish texts are the ones the app always had.
//! French puts a non-breaking space (`\u{a0}`) before `: ; ? !` and inside « ».
//! Functions here take the [`Locale`]; only the command boundary reads
//! [`crate::i18n::current`].

use plexustyping_ai::PlanMessage;
use plexustyping_ai::chatgpt_auth::{AuthError, CREDENTIALS_FILE};
use plexustyping_store::{PROFILE_NAME_MAX, StoreError};

use crate::i18n::{Locale, Texts};

// --- AI: keychain, providers, settings and notices -------------------------

pub const KEYCHAIN_UNAVAILABLE: Texts = Texts {
    es: "Llavero no disponible: las claves nunca se guardan en texto plano",
    en: "Keychain unavailable: keys are never stored in plain text",
    pt: "Chaveiro indisponível: as chaves nunca são guardadas em texto simples",
    fr: "Trousseau indisponible\u{a0}: les clés ne sont jamais enregistrées en clair",
    de: "Schlüsselbund nicht verfügbar: Schlüssel werden nie im Klartext gespeichert",
};

pub const KEYCHAIN_ERROR: Texts = Texts {
    es: "Error del llavero del sistema: {message}",
    en: "System keychain error: {message}",
    pt: "Erro do chaveiro do sistema: {message}",
    fr: "Erreur du trousseau système\u{a0}: {message}",
    de: "Fehler im Schlüsselbund des Systems: {message}",
};

pub const KEYCHAIN_SHORT: Texts = Texts {
    es: "Llavero no disponible",
    en: "Keychain unavailable",
    pt: "Chaveiro indisponível",
    fr: "Trousseau indisponible",
    de: "Schlüsselbund nicht verfügbar",
};

pub const NO_KEY: Texts = Texts {
    es: "Falta la clave de {name}",
    en: "The {name} key is missing",
    pt: "Falta a chave de {name}",
    fr: "La clé {name} est manquante",
    de: "Der Schlüssel für {name} fehlt",
};

pub const KEY_REJECTED: Texts = Texts {
    es: "{name} rechazó la clave",
    en: "{name} rejected the key",
    pt: "{name} rejeitou a chave",
    fr: "{name} a refusé la clé",
    de: "{name} hat den Schlüssel abgelehnt",
};

pub const RATE_LIMITED: Texts = Texts {
    es: "{name} está limitando las peticiones",
    en: "{name} is limiting requests",
    pt: "{name} está limitando as solicitações",
    fr: "{name} limite les requêtes",
    de: "{name} begrenzt die Anfragen",
};

pub const NO_CONNECTION: Texts = Texts {
    es: "Sin conexión con la IA",
    en: "No connection to the AI",
    pt: "Sem conexão com a IA",
    fr: "Pas de connexion à l’IA",
    de: "Keine Verbindung zur KI",
};

pub const TIMED_OUT: Texts = Texts {
    es: "{name} no respondió a tiempo",
    en: "{name} did not answer in time",
    pt: "{name} não respondeu a tempo",
    fr: "{name} n’a pas répondu à temps",
    de: "{name} hat nicht rechtzeitig geantwortet",
};

pub const REFUSED: Texts = Texts {
    es: "{name} se negó a generar el texto",
    en: "{name} refused to generate the text",
    pt: "{name} se recusou a gerar o texto",
    fr: "{name} a refusé de générer le texte",
    de: "{name} hat sich geweigert, den Text zu erzeugen",
};

pub const BAD_TEXT: Texts = Texts {
    es: "La IA no devolvió un texto válido",
    en: "The AI did not return a valid text",
    pt: "A IA não devolveu um texto válido",
    fr: "L’IA n’a pas renvoyé de texte valide",
    de: "Die KI hat keinen gültigen Text geliefert",
};

pub const REQUEST_REJECTED: Texts = Texts {
    es: "{name} rechazó la petición (revisa el modelo)",
    en: "{name} rejected the request (check the model)",
    pt: "{name} rejeitou a solicitação (verifique o modelo)",
    fr: "{name} a refusé la requête (vérifiez le modèle)",
    de: "{name} hat die Anfrage abgelehnt (Modell prüfen)",
};

pub const CHAIN_SIZE: Texts = Texts {
    es: "La cadena de respaldo necesita de 1 a {max} proveedores",
    en: "The fallback chain needs 1 to {max} providers",
    pt: "A cadeia de reserva precisa de 1 a {max} provedores",
    fr: "La chaîne de secours nécessite de 1 à {max} fournisseurs",
    de: "Die Ausweichkette braucht 1 bis {max} Anbieter",
};

pub const MODEL_TOO_LONG: Texts = Texts {
    es: "El nombre del modelo es demasiado largo",
    en: "The model name is too long",
    pt: "O nome do modelo é longo demais",
    fr: "Le nom du modèle est trop long",
    de: "Der Modellname ist zu lang",
};

pub const CHAIN_DUPLICATE: Texts = Texts {
    es: "{name} aparece dos veces en la cadena de respaldo",
    en: "{name} appears twice in the fallback chain",
    pt: "{name} aparece duas vezes na cadeia de reserva",
    fr: "{name} apparaît deux fois dans la chaîne de secours",
    de: "{name} kommt zweimal in der Ausweichkette vor",
};

pub const DENSITY_RANGE: Texts = Texts {
    es: "La densidad debe estar entre 2× y 5×",
    en: "The density must be between 2× and 5×",
    pt: "A densidade deve estar entre 2× e 5×",
    fr: "La densité doit être comprise entre 2× et 5×",
    de: "Die Dichte muss zwischen 2× und 5× liegen",
};

pub const TOPIC_TOO_LONG: Texts = Texts {
    es: "El tema admite como máximo {max} caracteres",
    en: "The topic can have at most {max} characters",
    pt: "O tema aceita no máximo {max} caracteres",
    fr: "Le thème accepte au plus {max} caractères",
    de: "Das Thema darf höchstens {max} Zeichen haben",
};

pub const TOPIC_LINE_BREAKS: Texts = Texts {
    es: "El tema no puede tener saltos de línea",
    en: "The topic cannot have line breaks",
    pt: "O tema não pode ter quebras de linha",
    fr: "Le thème ne peut pas contenir de retours à la ligne",
    de: "Das Thema darf keine Zeilenumbrüche enthalten",
};

pub const SAVE_FAILED: Texts = Texts {
    es: "No se pudo guardar: {error}",
    en: "Could not save: {error}",
    pt: "Não foi possível salvar: {error}",
    fr: "Impossible d’enregistrer\u{a0}: {error}",
    de: "Speichern fehlgeschlagen: {error}",
};

pub const CHATGPT_NO_KEY: Texts = Texts {
    es: "ChatGPT no usa clave: inicia sesión con ChatGPT en Ajustes → IA",
    en: "ChatGPT uses no key: sign in with ChatGPT in Settings → AI",
    pt: "O ChatGPT não usa chave: entre com o ChatGPT em Configurações → IA",
    fr: "ChatGPT n’utilise pas de clé\u{a0}: connectez-vous avec ChatGPT dans Paramètres → IA",
    de: "ChatGPT nutzt keinen Schlüssel: Melde dich unter Einstellungen → KI bei ChatGPT an",
};

pub const EMPTY_KEY: Texts = Texts {
    es: "La clave está vacía",
    en: "The key is empty",
    pt: "A chave está vazia",
    fr: "La clé est vide",
    de: "Der Schlüssel ist leer",
};

pub const MISSING_MODEL: Texts = Texts {
    es: "Falta el modelo de {name}",
    en: "The {name} model is missing",
    pt: "Falta o modelo de {name}",
    fr: "Le modèle {name} est manquant",
    de: "Das Modell für {name} fehlt",
};

pub const CHAIN_EMPTY: Texts = Texts {
    es: "La cadena de respaldo está vacía",
    en: "The fallback chain is empty",
    pt: "A cadeia de reserva está vazia",
    fr: "La chaîne de secours est vide",
    de: "Die Ausweichkette ist leer",
};

pub const NO_ANSWER: Texts = Texts {
    es: "La IA no respondió",
    en: "The AI did not answer",
    pt: "A IA não respondeu",
    fr: "L’IA n’a pas répondu",
    de: "Die KI hat nicht geantwortet",
};

pub const FROM_CACHE: Texts = Texts {
    es: "{reason}: texto del caché",
    en: "{reason}: text from the cache",
    pt: "{reason}: texto do cache",
    fr: "{reason}\u{a0}: texte du cache",
    de: "{reason}: Text aus dem Cache",
};

pub const FROM_CORPUS: Texts = Texts {
    es: "{reason}: texto del corpus local",
    en: "{reason}: text from the local corpus",
    pt: "{reason}: texto do corpus local",
    fr: "{reason}\u{a0}: texte du corpus local",
    de: "{reason}: Text aus dem lokalen Korpus",
};

pub const TAIL_TOO_LONG: Texts = Texts {
    es: "El texto a continuar admite como máximo {max} caracteres",
    en: "The text to continue can have at most {max} characters",
    pt: "O texto a continuar aceita no máximo {max} caracteres",
    fr: "Le texte à poursuivre accepte au plus {max} caractères",
    de: "Der fortzusetzende Text darf höchstens {max} Zeichen haben",
};

// --- ChatGPT plan messages (plexustyping-ai's `PlanMessage`) ---------------

pub const PLAN_USAGE_LIMIT: Texts = Texts {
    es: "Has alcanzado el límite de tu plan de ChatGPT",
    en: "You have reached the limit of your ChatGPT plan",
    pt: "Você atingiu o limite do seu plano do ChatGPT",
    fr: "Vous avez atteint la limite de votre forfait ChatGPT",
    de: "Du hast das Limit deines ChatGPT-Plans erreicht",
};

pub const PLAN_USAGE_UNAVAILABLE: Texts = Texts {
    es: "ChatGPT no pudo comprobar el uso de tu plan ahora; inténtalo más tarde",
    en: "ChatGPT could not check your plan’s usage right now; try again later",
    pt: "O ChatGPT não conseguiu verificar o uso do seu plano agora; tente mais tarde",
    fr: "ChatGPT n’a pas pu vérifier l’utilisation de votre forfait pour le moment\u{a0}; réessayez plus tard",
    de: "ChatGPT konnte die Nutzung deines Plans gerade nicht prüfen; versuche es später erneut",
};

pub const PLAN_NOT_ELIGIBLE: Texts = Texts {
    es: "Tu plan de ChatGPT no se puede usar en PlexusTyping (cuenta, espacio de trabajo o política)",
    en: "Your ChatGPT plan cannot be used in PlexusTyping (account, workspace or policy)",
    pt: "Seu plano do ChatGPT não pode ser usado no PlexusTyping (conta, espaço de trabalho ou política)",
    fr: "Votre forfait ChatGPT ne peut pas être utilisé dans PlexusTyping (compte, espace de travail ou politique)",
    de: "Dein ChatGPT-Plan kann in PlexusTyping nicht genutzt werden (Konto, Arbeitsbereich oder Richtlinie)",
};

pub const PLAN_UNSUPPORTED: Texts = Texts {
    es: "ChatGPT no admite \"{param}\" en esta petición. Prueba con otro modelo",
    en: "ChatGPT does not support \"{param}\" in this request. Try another model",
    pt: "O ChatGPT não aceita \"{param}\" nesta solicitação. Tente outro modelo",
    fr: "ChatGPT ne prend pas en charge «\u{a0}{param}\u{a0}» dans cette requête. Essayez un autre modèle",
    de: "ChatGPT unterstützt „{param}“ in dieser Anfrage nicht. Versuche ein anderes Modell",
};

pub const PLAN_UNSUPPORTED_REQUEST: Texts = Texts {
    es: "ChatGPT no admite esta petición. Prueba con otro modelo",
    en: "ChatGPT does not support this request. Try another model",
    pt: "O ChatGPT não aceita esta solicitação. Tente outro modelo",
    fr: "ChatGPT ne prend pas en charge cette requête. Essayez un autre modèle",
    de: "ChatGPT unterstützt diese Anfrage nicht. Versuche ein anderes Modell",
};

pub const PLAN_NOT_AUTHORIZED: Texts = Texts {
    es: "ChatGPT no autorizó esta petición de PlexusTyping",
    en: "ChatGPT did not authorize this request from PlexusTyping",
    pt: "O ChatGPT não autorizou esta solicitação do PlexusTyping",
    fr: "ChatGPT n’a pas autorisé cette requête de PlexusTyping",
    de: "ChatGPT hat diese Anfrage von PlexusTyping nicht autorisiert",
};

pub const PLAN_INVALID_USER: Texts = Texts {
    es: "ChatGPT no pudo validar tu sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT",
    en: "ChatGPT could not validate your session. If it keeps failing, sign in to ChatGPT again",
    pt: "O ChatGPT não conseguiu validar sua sessão. Se continuar falhando, entre de novo no ChatGPT",
    fr: "ChatGPT n’a pas pu valider votre session. Si le problème persiste, reconnectez-vous à ChatGPT",
    de: "ChatGPT konnte deine Sitzung nicht prüfen. Wenn es weiter fehlschlägt, melde dich erneut bei ChatGPT an",
};

pub const PLAN_SESSION_REJECTED: Texts = Texts {
    es: "ChatGPT no aceptó la sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT",
    en: "ChatGPT did not accept the session. If it keeps failing, sign in to ChatGPT again",
    pt: "O ChatGPT não aceitou a sessão. Se continuar falhando, entre de novo no ChatGPT",
    fr: "ChatGPT n’a pas accepté la session. Si le problème persiste, reconnectez-vous à ChatGPT",
    de: "ChatGPT hat die Sitzung nicht akzeptiert. Wenn es weiter fehlschlägt, melde dich erneut bei ChatGPT an",
};

pub const PLAN_FORBIDDEN: Texts = Texts {
    es: "ChatGPT no permitió esta petición (región, permisos o política)",
    en: "ChatGPT did not allow this request (region, permissions or policy)",
    pt: "O ChatGPT não permitiu esta solicitação (região, permissões ou política)",
    fr: "ChatGPT n’a pas autorisé cette requête (région, autorisations ou politique)",
    de: "ChatGPT hat diese Anfrage nicht erlaubt (Region, Berechtigungen oder Richtlinie)",
};

pub const PLAN_REDIRECTED: Texts = Texts {
    es: "ChatGPT redirigió la petición (HTTP {status}); no se siguen redirecciones",
    en: "ChatGPT redirected the request (HTTP {status}); redirects are not followed",
    pt: "O ChatGPT redirecionou a solicitação (HTTP {status}); redirecionamentos não são seguidos",
    fr: "ChatGPT a redirigé la requête (HTTP {status})\u{a0}; les redirections ne sont pas suivies",
    de: "ChatGPT hat die Anfrage umgeleitet (HTTP {status}); Umleitungen werden nicht verfolgt",
};

pub const PLAN_THROTTLED: Texts = Texts {
    es: "ChatGPT está limitando las peticiones; inténtalo en un momento",
    en: "ChatGPT is limiting requests; try again in a moment",
    pt: "O ChatGPT está limitando as solicitações; tente daqui a pouco",
    fr: "ChatGPT limite les requêtes\u{a0}; réessayez dans un instant",
    de: "ChatGPT begrenzt die Anfragen; versuche es gleich noch einmal",
};

pub const PLAN_REJECTED: Texts = Texts {
    es: "ChatGPT rechazó la petición (HTTP {status}, {code}). Revisa el modelo",
    en: "ChatGPT rejected the request (HTTP {status}, {code}). Check the model",
    pt: "O ChatGPT rejeitou a solicitação (HTTP {status}, {code}). Verifique o modelo",
    fr: "ChatGPT a refusé la requête (HTTP {status}, {code}). Vérifiez le modèle",
    de: "ChatGPT hat die Anfrage abgelehnt (HTTP {status}, {code}). Prüfe das Modell",
};

pub const PLAN_REJECTED_STATUS: Texts = Texts {
    es: "ChatGPT rechazó la petición (HTTP {status}). Revisa el modelo",
    en: "ChatGPT rejected the request (HTTP {status}). Check the model",
    pt: "O ChatGPT rejeitou a solicitação (HTTP {status}). Verifique o modelo",
    fr: "ChatGPT a refusé la requête (HTTP {status}). Vérifiez le modèle",
    de: "ChatGPT hat die Anfrage abgelehnt (HTTP {status}). Prüfe das Modell",
};

pub const PLAN_NOT_ENABLED: Texts = Texts {
    es: "Tu cuenta de ChatGPT no tiene activado el uso del plan",
    en: "Your ChatGPT account does not have plan use enabled",
    pt: "Sua conta do ChatGPT não tem o uso do plano ativado",
    fr: "L’utilisation du forfait n’est pas activée sur votre compte ChatGPT",
    de: "Für dein ChatGPT-Konto ist die Nutzung des Plans nicht aktiviert",
};

// --- Sign in with ChatGPT (plexustyping-ai's `AuthError`) ------------------

pub const AUTH_NO_STORAGE: Texts = Texts {
    es: "PlexusTyping no tiene una carpeta donde guardar la sesión de ChatGPT",
    en: "PlexusTyping has no folder to keep the ChatGPT session in",
    pt: "O PlexusTyping não tem uma pasta onde guardar a sessão do ChatGPT",
    fr: "PlexusTyping n’a aucun dossier où enregistrer la session ChatGPT",
    de: "PlexusTyping hat keinen Ordner, um die ChatGPT-Sitzung zu speichern",
};

pub const AUTH_STORAGE: Texts = Texts {
    es: "No se pudo guardar o leer la sesión de ChatGPT: {message}",
    en: "Could not save or read the ChatGPT session: {message}",
    pt: "Não foi possível salvar ou ler a sessão do ChatGPT: {message}",
    fr: "Impossible d’enregistrer ou de lire la session ChatGPT\u{a0}: {message}",
    de: "Die ChatGPT-Sitzung konnte nicht gespeichert oder gelesen werden: {message}",
};

pub const AUTH_DAMAGED_FILE: Texts = Texts {
    es: "El archivo de sesión de ChatGPT ({file}) está dañado. Usa «Cerrar sesión» para empezar de nuevo",
    en: "The ChatGPT session file ({file}) is damaged. Use “Sign out” to start again",
    pt: "O arquivo de sessão do ChatGPT ({file}) está danificado. Use “Sair” para começar de novo",
    fr: "Le fichier de session ChatGPT ({file}) est endommagé. Utilisez «\u{a0}Se déconnecter\u{a0}» pour recommencer",
    de: "Die ChatGPT-Sitzungsdatei ({file}) ist beschädigt. Mit „Abmelden“ fängst du neu an",
};

pub const AUTH_LISTENER: Texts = Texts {
    es: "No se pudo preparar la respuesta del navegador: {message}",
    en: "Could not get ready for the browser’s reply: {message}",
    pt: "Não foi possível preparar a resposta do navegador: {message}",
    fr: "Impossible de préparer la réponse du navigateur\u{a0}: {message}",
    de: "Die Antwort des Browsers konnte nicht vorbereitet werden: {message}",
};

pub const AUTH_BROWSER: Texts = Texts {
    es: "No se pudo abrir el navegador",
    en: "Could not open the browser",
    pt: "Não foi possível abrir o navegador",
    fr: "Impossible d’ouvrir le navigateur",
    de: "Der Browser konnte nicht geöffnet werden",
};

pub const AUTH_CANCELLED: Texts = Texts {
    es: "Se canceló el inicio de sesión en ChatGPT",
    en: "The ChatGPT sign-in was cancelled",
    pt: "O login no ChatGPT foi cancelado",
    fr: "La connexion à ChatGPT a été annulée",
    de: "Die Anmeldung bei ChatGPT wurde abgebrochen",
};

pub const AUTH_TIMEOUT: Texts = Texts {
    es: "El inicio de sesión en ChatGPT ha caducado. Inténtalo de nuevo",
    en: "The ChatGPT sign-in timed out. Try again",
    pt: "O login no ChatGPT expirou. Tente de novo",
    fr: "La connexion à ChatGPT a expiré. Réessayez",
    de: "Die Anmeldung bei ChatGPT ist abgelaufen. Versuche es erneut",
};

pub const AUTH_ACCESS_DENIED: Texts = Texts {
    es: "Has rechazado el permiso, así que PlexusTyping no puede usar tu plan de ChatGPT",
    en: "You declined the permission, so PlexusTyping cannot use your ChatGPT plan",
    pt: "Você recusou a permissão, então o PlexusTyping não pode usar seu plano do ChatGPT",
    fr: "Vous avez refusé l’autorisation, PlexusTyping ne peut donc pas utiliser votre forfait ChatGPT",
    de: "Du hast die Berechtigung abgelehnt, daher kann PlexusTyping deinen ChatGPT-Plan nicht nutzen",
};

pub const AUTH_AUTHORIZATION: Texts = Texts {
    es: "ChatGPT no pudo completar el inicio de sesión ({code})",
    en: "ChatGPT could not complete the sign-in ({code})",
    pt: "O ChatGPT não conseguiu concluir o login ({code})",
    fr: "ChatGPT n’a pas pu terminer la connexion ({code})",
    de: "ChatGPT konnte die Anmeldung nicht abschließen ({code})",
};

pub const AUTH_MISSING_CODE: Texts = Texts {
    es: "ChatGPT no devolvió un código de autorización",
    en: "ChatGPT returned no authorization code",
    pt: "O ChatGPT não devolveu um código de autorização",
    fr: "ChatGPT n’a renvoyé aucun code d’autorisation",
    de: "ChatGPT hat keinen Autorisierungscode zurückgegeben",
};

pub const AUTH_MISSING_CLIENT_ID: Texts = Texts {
    es: "ChatGPT no terminó de registrar PlexusTyping. Inténtalo de nuevo",
    en: "ChatGPT did not finish registering PlexusTyping. Try again",
    pt: "O ChatGPT não terminou de registrar o PlexusTyping. Tente de novo",
    fr: "ChatGPT n’a pas terminé l’enregistrement de PlexusTyping. Réessayez",
    de: "ChatGPT hat die Registrierung von PlexusTyping nicht abgeschlossen. Versuche es erneut",
};

pub const AUTH_CLIENT_ID_MISMATCH: Texts = Texts {
    es: "ChatGPT devolvió otro registro de la aplicación; no se ha usado",
    en: "ChatGPT returned another registration of the app; it was not used",
    pt: "O ChatGPT devolveu outro registro do aplicativo; ele não foi usado",
    fr: "ChatGPT a renvoyé un autre enregistrement de l’application\u{a0}; il n’a pas été utilisé",
    de: "ChatGPT hat eine andere Registrierung der App zurückgegeben; sie wurde nicht verwendet",
};

pub const AUTH_CODE_EXCHANGE: Texts = Texts {
    es: "No se pudo canjear el código de ChatGPT ({detail}). Inténtalo de nuevo",
    en: "Could not redeem the ChatGPT code ({detail}). Try again",
    pt: "Não foi possível trocar o código do ChatGPT ({detail}). Tente de novo",
    fr: "Impossible d’échanger le code ChatGPT ({detail}). Réessayez",
    de: "Der ChatGPT-Code konnte nicht eingelöst werden ({detail}). Versuche es erneut",
};

pub const AUTH_KEYS_UNAVAILABLE: Texts = Texts {
    es: "No se pudieron obtener las claves de OpenAI para verificar el inicio de sesión. Comprueba la conexión e inténtalo de nuevo",
    en: "Could not get OpenAI’s keys to verify the sign-in. Check the connection and try again",
    pt: "Não foi possível obter as chaves da OpenAI para verificar o login. Verifique a conexão e tente de novo",
    fr: "Impossible d’obtenir les clés d’OpenAI pour vérifier la connexion. Vérifiez votre réseau et réessayez",
    de: "Die Schlüssel von OpenAI zum Prüfen der Anmeldung waren nicht abrufbar. Prüfe die Verbindung und versuche es erneut",
};

pub const AUTH_INVALID_ID_TOKEN: Texts = Texts {
    es: "ChatGPT envió un inicio de sesión que PlexusTyping no pudo verificar ({reason})",
    en: "ChatGPT sent a sign-in that PlexusTyping could not verify ({reason})",
    pt: "O ChatGPT enviou um login que o PlexusTyping não conseguiu verificar ({reason})",
    fr: "ChatGPT a envoyé une connexion que PlexusTyping n’a pas pu vérifier ({reason})",
    de: "ChatGPT hat eine Anmeldung gesendet, die PlexusTyping nicht prüfen konnte ({reason})",
};

pub const AUTH_IDENTITY_MISMATCH: Texts = Texts {
    es: "Ha iniciado sesión otra cuenta de ChatGPT distinta de la registrada; no se ha usado",
    en: "A different ChatGPT account than the registered one signed in; it was not used",
    pt: "Entrou uma conta do ChatGPT diferente da registrada; ela não foi usada",
    fr: "Un autre compte ChatGPT que celui enregistré s’est connecté\u{a0}; il n’a pas été utilisé",
    de: "Ein anderes ChatGPT-Konto als das registrierte hat sich angemeldet; es wurde nicht verwendet",
};

pub const AUTH_NOT_SIGNED_IN: Texts = Texts {
    es: "Inicia sesión en ChatGPT",
    en: "Sign in to ChatGPT",
    pt: "Entre no ChatGPT",
    fr: "Connectez-vous à ChatGPT",
    de: "Melde dich bei ChatGPT an",
};

pub const AUTH_PLAN_NOT_ENABLED: Texts = Texts {
    es: "El uso de tu plan de ChatGPT no está activado para PlexusTyping. Inicia sesión de nuevo y permítelo",
    en: "Use of your ChatGPT plan is not enabled for PlexusTyping. Sign in again and allow it",
    pt: "O uso do seu plano do ChatGPT não está ativado para o PlexusTyping. Entre de novo e permita",
    fr: "L’utilisation de votre forfait ChatGPT n’est pas activée pour PlexusTyping. Reconnectez-vous et autorisez-la",
    de: "Die Nutzung deines ChatGPT-Plans ist für PlexusTyping nicht aktiviert. Melde dich erneut an und erlaube sie",
};

pub const AUTH_SESSION_EXPIRED: Texts = Texts {
    es: "Tu sesión de ChatGPT ha caducado. Vuelve a iniciar sesión",
    en: "Your ChatGPT session has expired. Sign in again",
    pt: "Sua sessão do ChatGPT expirou. Entre de novo",
    fr: "Votre session ChatGPT a expiré. Reconnectez-vous",
    de: "Deine ChatGPT-Sitzung ist abgelaufen. Melde dich erneut an",
};

pub const AUTH_INVALID_CLIENT: Texts = Texts {
    es: "ChatGPT ya no acepta el registro de PlexusTyping. Cierra sesión y vuelve a iniciarla",
    en: "ChatGPT no longer accepts PlexusTyping’s registration. Sign out and sign in again",
    pt: "O ChatGPT não aceita mais o registro do PlexusTyping. Saia e entre de novo",
    fr: "ChatGPT n’accepte plus l’enregistrement de PlexusTyping. Déconnectez-vous puis reconnectez-vous",
    de: "ChatGPT akzeptiert die Registrierung von PlexusTyping nicht mehr. Melde dich ab und wieder an",
};

pub const AUTH_REFRESH_REFUSED: Texts = Texts {
    es: "ChatGPT no renovó la sesión ({detail})",
    en: "ChatGPT did not renew the session ({detail})",
    pt: "O ChatGPT não renovou a sessão ({detail})",
    fr: "ChatGPT n’a pas renouvelé la session ({detail})",
    de: "ChatGPT hat die Sitzung nicht erneuert ({detail})",
};

pub const AUTH_UNAVAILABLE: Texts = Texts {
    es: "El servicio de inicio de sesión de ChatGPT no está disponible ahora (HTTP {status})",
    en: "The ChatGPT sign-in service is unavailable right now (HTTP {status})",
    pt: "O serviço de login do ChatGPT não está disponível agora (HTTP {status})",
    fr: "Le service de connexion de ChatGPT n’est pas disponible pour le moment (HTTP {status})",
    de: "Der Anmeldedienst von ChatGPT ist gerade nicht verfügbar (HTTP {status})",
};

pub const AUTH_NETWORK: Texts = Texts {
    es: "No se pudo conectar con auth.openai.com: {message}",
    en: "Could not reach auth.openai.com: {message}",
    pt: "Não foi possível conectar a auth.openai.com: {message}",
    fr: "Impossible de joindre auth.openai.com\u{a0}: {message}",
    de: "auth.openai.com war nicht erreichbar: {message}",
};

pub const AUTH_BAD_RESPONSE: Texts = Texts {
    es: "Respuesta inesperada de auth.openai.com",
    en: "Unexpected reply from auth.openai.com",
    pt: "Resposta inesperada de auth.openai.com",
    fr: "Réponse inattendue de auth.openai.com",
    de: "Unerwartete Antwort von auth.openai.com",
};

// --- History and goals ------------------------------------------------------

pub const NO_SESSIONS: Texts = Texts {
    es: "No hay sesiones que exportar.",
    en: "There are no sessions to export.",
    pt: "Não há sessões para exportar.",
    fr: "Il n’y a aucune session à exporter.",
    de: "Es gibt keine Sitzungen zum Exportieren.",
};

pub const UNKNOWN_MODE: Texts = Texts {
    es: "Modo desconocido: {mode}",
    en: "Unknown mode: {mode}",
    pt: "Modo desconhecido: {mode}",
    fr: "Mode inconnu\u{a0}: {mode}",
    de: "Unbekannter Modus: {mode}",
};

pub const INVALID_TZ: Texts = Texts {
    es: "Zona horaria no válida: {offset}",
    en: "Invalid time zone: {offset}",
    pt: "Fuso horário inválido: {offset}",
    fr: "Fuseau horaire non valide\u{a0}: {offset}",
    de: "Ungültige Zeitzone: {offset}",
};

pub const CSV_FAILED: Texts = Texts {
    es: "No se pudo exportar el CSV: {error}",
    en: "Could not export the CSV: {error}",
    pt: "Não foi possível exportar o CSV: {error}",
    fr: "Impossible d’exporter le CSV\u{a0}: {error}",
    de: "CSV-Export fehlgeschlagen: {error}",
};

pub const CSV_NO_FREE_NAME: Texts = Texts {
    es: "No se pudo exportar el CSV: no hay nombre libre",
    en: "Could not export the CSV: no free file name",
    pt: "Não foi possível exportar o CSV: não há nome livre",
    fr: "Impossible d’exporter le CSV\u{a0}: aucun nom de fichier libre",
    de: "CSV-Export fehlgeschlagen: kein freier Dateiname",
};

pub const INVALID_GOAL: Texts = Texts {
    es: "Meta diaria no válida",
    en: "Invalid daily goal",
    pt: "Meta diária inválida",
    fr: "Objectif quotidien non valide",
    de: "Ungültiges Tagesziel",
};

// --- Profiles ---------------------------------------------------------------

pub const UNKNOWN_LANGUAGE: Texts = Texts {
    es: "Idioma desconocido",
    en: "Unknown language",
    pt: "Idioma desconhecido",
    fr: "Langue inconnue",
    de: "Unbekannte Sprache",
};

pub const UNKNOWN_LAYOUT: Texts = Texts {
    es: "Distribución desconocida",
    en: "Unknown layout",
    pt: "Layout desconhecido",
    fr: "Disposition inconnue",
    de: "Unbekannte Tastaturbelegung",
};

pub const EMPTY_PROFILE_NAME: Texts = Texts {
    es: "El nombre del perfil no puede estar vacío",
    en: "The profile name cannot be empty",
    pt: "O nome do perfil não pode ficar vazio",
    fr: "Le nom du profil ne peut pas être vide",
    de: "Der Profilname darf nicht leer sein",
};

pub const PROFILE_NAME_TOO_LONG: Texts = Texts {
    es: "El nombre del perfil admite como máximo {max} caracteres",
    en: "The profile name can have at most {max} characters",
    pt: "O nome do perfil aceita no máximo {max} caracteres",
    fr: "Le nom du profil accepte au plus {max} caractères",
    de: "Der Profilname darf höchstens {max} Zeichen haben",
};

pub const DUPLICATE_PROFILE_NAME: Texts = Texts {
    es: "Ya existe un perfil con ese nombre",
    en: "A profile with that name already exists",
    pt: "Já existe um perfil com esse nome",
    fr: "Un profil porte déjà ce nom",
    de: "Ein Profil mit diesem Namen gibt es bereits",
};

pub const LAST_PROFILE: Texts = Texts {
    es: "No se puede borrar el único perfil",
    en: "The only profile cannot be deleted",
    pt: "Não é possível excluir o único perfil",
    fr: "Impossible de supprimer le seul profil",
    de: "Das einzige Profil kann nicht gelöscht werden",
};

pub const PROFILE_NOT_FOUND: Texts = Texts {
    es: "Ese perfil no existe",
    en: "That profile does not exist",
    pt: "Esse perfil não existe",
    fr: "Ce profil n’existe pas",
    de: "Dieses Profil existiert nicht",
};

// --- Progressive unlock, metronome and UI settings --------------------------

pub const INVALID_TARGET: Texts = Texts {
    es: "Velocidad objetivo no válida",
    en: "Invalid target speed",
    pt: "Velocidade-alvo inválida",
    fr: "Vitesse cible non valide",
    de: "Ungültige Zielgeschwindigkeit",
};

pub const METRONOME_INVALID: Texts = Texts {
    es: "Configuración del metrónomo no válida: tempo de 30 a 300 BPM, volumen de 0 a 100 % y acento cada 0 a 16 pulsaciones",
    en: "Invalid metronome settings: tempo from 30 to 300 BPM, volume from 0 to 100% and an accent every 0 to 16 beats",
    pt: "Configuração do metrônomo inválida: andamento de 30 a 300 BPM, volume de 0 a 100% e acento a cada 0 a 16 batidas",
    fr: "Réglages du métronome non valides\u{a0}: tempo de 30 à 300\u{a0}BPM, volume de 0 à 100\u{a0}% et accent toutes les 0 à 16 pulsations",
    de: "Ungültige Metronom-Einstellungen: Tempo 30 bis 300 BPM, Lautstärke 0 bis 100 % und Akzent alle 0 bis 16 Schläge",
};

pub const NO_AUDIO_DEVICE: Texts = Texts {
    es: "No hay dispositivo de audio",
    en: "No audio device",
    pt: "Nenhum dispositivo de áudio",
    fr: "Aucun périphérique audio",
    de: "Kein Audiogerät",
};

pub const AUDIO_FAILED: Texts = Texts {
    es: "No se pudo reproducir el audio: {detail}",
    en: "Could not play the audio: {detail}",
    pt: "Não foi possível reproduzir o áudio: {detail}",
    fr: "Impossible de lire l’audio\u{a0}: {detail}",
    de: "Audio konnte nicht abgespielt werden: {detail}",
};

pub const METRONOME_INTERNAL: Texts = Texts {
    es: "Error interno del metrónomo: {error}",
    en: "Internal metronome error: {error}",
    pt: "Erro interno do metrônomo: {error}",
    fr: "Erreur interne du métronome\u{a0}: {error}",
    de: "Interner Fehler des Metronoms: {error}",
};

pub const UI_SETTINGS_TEMPORARY: Texts = Texts {
    es: "La base de datos no está disponible: los ajustes siguen en el almacenamiento del navegador",
    en: "The database is unavailable: the settings stay in the browser’s storage",
    pt: "O banco de dados não está disponível: os ajustes continuam no armazenamento do navegador",
    fr: "La base de données n’est pas disponible\u{a0}: les réglages restent dans le stockage du navigateur",
    de: "Die Datenbank ist nicht verfügbar: Die Einstellungen bleiben im Speicher des Browsers",
};

pub const INVALID_SETTING: Texts = Texts {
    es: "Ajuste no válido: «{key}»",
    en: "Invalid setting: “{key}”",
    pt: "Ajuste inválido: “{key}”",
    fr: "Réglage non valide\u{a0}: «\u{a0}{key}\u{a0}»",
    de: "Ungültige Einstellung: „{key}“",
};

pub const SETTING_TOO_LARGE: Texts = Texts {
    es: "El ajuste «{key}» es demasiado grande (máximo {max} KiB)",
    en: "The setting “{key}” is too large (at most {max} KiB)",
    pt: "O ajuste “{key}” é grande demais (máximo de {max} KiB)",
    fr: "Le réglage «\u{a0}{key}\u{a0}» est trop volumineux (maximum {max}\u{a0}Kio)",
    de: "Die Einstellung „{key}“ ist zu groß (höchstens {max} KiB)",
};

/// Every catalog entry, for the placeholder check (S5).
pub const CATALOG: &[(&str, Texts)] = &[
    ("KEYCHAIN_UNAVAILABLE", KEYCHAIN_UNAVAILABLE),
    ("KEYCHAIN_ERROR", KEYCHAIN_ERROR),
    ("KEYCHAIN_SHORT", KEYCHAIN_SHORT),
    ("NO_KEY", NO_KEY),
    ("KEY_REJECTED", KEY_REJECTED),
    ("RATE_LIMITED", RATE_LIMITED),
    ("NO_CONNECTION", NO_CONNECTION),
    ("TIMED_OUT", TIMED_OUT),
    ("REFUSED", REFUSED),
    ("BAD_TEXT", BAD_TEXT),
    ("REQUEST_REJECTED", REQUEST_REJECTED),
    ("CHAIN_SIZE", CHAIN_SIZE),
    ("MODEL_TOO_LONG", MODEL_TOO_LONG),
    ("CHAIN_DUPLICATE", CHAIN_DUPLICATE),
    ("DENSITY_RANGE", DENSITY_RANGE),
    ("TOPIC_TOO_LONG", TOPIC_TOO_LONG),
    ("TOPIC_LINE_BREAKS", TOPIC_LINE_BREAKS),
    ("SAVE_FAILED", SAVE_FAILED),
    ("CHATGPT_NO_KEY", CHATGPT_NO_KEY),
    ("EMPTY_KEY", EMPTY_KEY),
    ("MISSING_MODEL", MISSING_MODEL),
    ("CHAIN_EMPTY", CHAIN_EMPTY),
    ("NO_ANSWER", NO_ANSWER),
    ("FROM_CACHE", FROM_CACHE),
    ("FROM_CORPUS", FROM_CORPUS),
    ("TAIL_TOO_LONG", TAIL_TOO_LONG),
    ("PLAN_USAGE_LIMIT", PLAN_USAGE_LIMIT),
    ("PLAN_USAGE_UNAVAILABLE", PLAN_USAGE_UNAVAILABLE),
    ("PLAN_NOT_ELIGIBLE", PLAN_NOT_ELIGIBLE),
    ("PLAN_UNSUPPORTED", PLAN_UNSUPPORTED),
    ("PLAN_UNSUPPORTED_REQUEST", PLAN_UNSUPPORTED_REQUEST),
    ("PLAN_NOT_AUTHORIZED", PLAN_NOT_AUTHORIZED),
    ("PLAN_INVALID_USER", PLAN_INVALID_USER),
    ("PLAN_SESSION_REJECTED", PLAN_SESSION_REJECTED),
    ("PLAN_FORBIDDEN", PLAN_FORBIDDEN),
    ("PLAN_REDIRECTED", PLAN_REDIRECTED),
    ("PLAN_THROTTLED", PLAN_THROTTLED),
    ("PLAN_REJECTED", PLAN_REJECTED),
    ("PLAN_REJECTED_STATUS", PLAN_REJECTED_STATUS),
    ("PLAN_NOT_ENABLED", PLAN_NOT_ENABLED),
    ("AUTH_NO_STORAGE", AUTH_NO_STORAGE),
    ("AUTH_STORAGE", AUTH_STORAGE),
    ("AUTH_DAMAGED_FILE", AUTH_DAMAGED_FILE),
    ("AUTH_LISTENER", AUTH_LISTENER),
    ("AUTH_BROWSER", AUTH_BROWSER),
    ("AUTH_CANCELLED", AUTH_CANCELLED),
    ("AUTH_TIMEOUT", AUTH_TIMEOUT),
    ("AUTH_ACCESS_DENIED", AUTH_ACCESS_DENIED),
    ("AUTH_AUTHORIZATION", AUTH_AUTHORIZATION),
    ("AUTH_MISSING_CODE", AUTH_MISSING_CODE),
    ("AUTH_MISSING_CLIENT_ID", AUTH_MISSING_CLIENT_ID),
    ("AUTH_CLIENT_ID_MISMATCH", AUTH_CLIENT_ID_MISMATCH),
    ("AUTH_CODE_EXCHANGE", AUTH_CODE_EXCHANGE),
    ("AUTH_KEYS_UNAVAILABLE", AUTH_KEYS_UNAVAILABLE),
    ("AUTH_INVALID_ID_TOKEN", AUTH_INVALID_ID_TOKEN),
    ("AUTH_IDENTITY_MISMATCH", AUTH_IDENTITY_MISMATCH),
    ("AUTH_NOT_SIGNED_IN", AUTH_NOT_SIGNED_IN),
    ("AUTH_PLAN_NOT_ENABLED", AUTH_PLAN_NOT_ENABLED),
    ("AUTH_SESSION_EXPIRED", AUTH_SESSION_EXPIRED),
    ("AUTH_INVALID_CLIENT", AUTH_INVALID_CLIENT),
    ("AUTH_REFRESH_REFUSED", AUTH_REFRESH_REFUSED),
    ("AUTH_UNAVAILABLE", AUTH_UNAVAILABLE),
    ("AUTH_NETWORK", AUTH_NETWORK),
    ("AUTH_BAD_RESPONSE", AUTH_BAD_RESPONSE),
    ("NO_SESSIONS", NO_SESSIONS),
    ("UNKNOWN_MODE", UNKNOWN_MODE),
    ("INVALID_TZ", INVALID_TZ),
    ("CSV_FAILED", CSV_FAILED),
    ("CSV_NO_FREE_NAME", CSV_NO_FREE_NAME),
    ("INVALID_GOAL", INVALID_GOAL),
    ("UNKNOWN_LANGUAGE", UNKNOWN_LANGUAGE),
    ("UNKNOWN_LAYOUT", UNKNOWN_LAYOUT),
    ("EMPTY_PROFILE_NAME", EMPTY_PROFILE_NAME),
    ("PROFILE_NAME_TOO_LONG", PROFILE_NAME_TOO_LONG),
    ("DUPLICATE_PROFILE_NAME", DUPLICATE_PROFILE_NAME),
    ("LAST_PROFILE", LAST_PROFILE),
    ("PROFILE_NOT_FOUND", PROFILE_NOT_FOUND),
    ("INVALID_TARGET", INVALID_TARGET),
    ("METRONOME_INVALID", METRONOME_INVALID),
    ("NO_AUDIO_DEVICE", NO_AUDIO_DEVICE),
    ("AUDIO_FAILED", AUDIO_FAILED),
    ("METRONOME_INTERNAL", METRONOME_INTERNAL),
    ("UI_SETTINGS_TEMPORARY", UI_SETTINGS_TEMPORARY),
    ("INVALID_SETTING", INVALID_SETTING),
    ("SETTING_TOO_LARGE", SETTING_TOO_LARGE),
];

/// A Sign in with ChatGPT failure in `locale`.
pub fn auth_error(locale: Locale, err: &AuthError) -> String {
    let (texts, params): (Texts, Vec<(&str, String)>) = match err {
        AuthError::NoStorage => (AUTH_NO_STORAGE, vec![]),
        AuthError::Storage { message } => (AUTH_STORAGE, vec![("message", message.clone())]),
        AuthError::DamagedFile => (AUTH_DAMAGED_FILE, vec![("file", CREDENTIALS_FILE.into())]),
        AuthError::Listener { message } => (AUTH_LISTENER, vec![("message", message.clone())]),
        AuthError::Browser => (AUTH_BROWSER, vec![]),
        AuthError::Cancelled => (AUTH_CANCELLED, vec![]),
        AuthError::Timeout => (AUTH_TIMEOUT, vec![]),
        AuthError::AccessDenied => (AUTH_ACCESS_DENIED, vec![]),
        AuthError::Authorization { code } => (AUTH_AUTHORIZATION, vec![("code", code.clone())]),
        AuthError::MissingCode => (AUTH_MISSING_CODE, vec![]),
        AuthError::MissingClientId => (AUTH_MISSING_CLIENT_ID, vec![]),
        AuthError::ClientIdMismatch => (AUTH_CLIENT_ID_MISMATCH, vec![]),
        AuthError::CodeExchange { detail } => {
            (AUTH_CODE_EXCHANGE, vec![("detail", detail.clone())])
        }
        AuthError::KeysUnavailable => (AUTH_KEYS_UNAVAILABLE, vec![]),
        AuthError::InvalidIdToken { reason } => {
            (AUTH_INVALID_ID_TOKEN, vec![("reason", reason.clone())])
        }
        AuthError::IdentityMismatch => (AUTH_IDENTITY_MISMATCH, vec![]),
        AuthError::NotSignedIn => (AUTH_NOT_SIGNED_IN, vec![]),
        AuthError::PlanNotEnabled => (AUTH_PLAN_NOT_ENABLED, vec![]),
        AuthError::SessionExpired => (AUTH_SESSION_EXPIRED, vec![]),
        AuthError::InvalidClient => (AUTH_INVALID_CLIENT, vec![]),
        AuthError::RefreshRefused { detail } => {
            (AUTH_REFRESH_REFUSED, vec![("detail", detail.clone())])
        }
        AuthError::Unavailable { status } => {
            (AUTH_UNAVAILABLE, vec![("status", status.to_string())])
        }
        AuthError::Network { message } => (AUTH_NETWORK, vec![("message", message.clone())]),
        AuthError::BadResponse => (AUTH_BAD_RESPONSE, vec![]),
    };
    fill(texts, locale, &params)
}

/// A ChatGPT plan message in `locale`.
pub fn plan_message(locale: Locale, message: &PlanMessage) -> String {
    let texts = match message {
        PlanMessage::UsageLimit => PLAN_USAGE_LIMIT,
        PlanMessage::UsageUnavailable => PLAN_USAGE_UNAVAILABLE,
        PlanMessage::NotEligible => PLAN_NOT_ELIGIBLE,
        PlanMessage::Unsupported { .. } => PLAN_UNSUPPORTED,
        PlanMessage::UnsupportedRequest => PLAN_UNSUPPORTED_REQUEST,
        PlanMessage::NotAuthorized => PLAN_NOT_AUTHORIZED,
        PlanMessage::InvalidUser => PLAN_INVALID_USER,
        PlanMessage::SessionRejected => PLAN_SESSION_REJECTED,
        PlanMessage::Forbidden => PLAN_FORBIDDEN,
        PlanMessage::Redirected { .. } => PLAN_REDIRECTED,
        PlanMessage::Throttled => PLAN_THROTTLED,
        PlanMessage::Rejected { .. } => PLAN_REJECTED,
        PlanMessage::RejectedStatus { .. } => PLAN_REJECTED_STATUS,
        PlanMessage::NotSignedIn => AUTH_NOT_SIGNED_IN,
        PlanMessage::PlanNotEnabled => PLAN_NOT_ENABLED,
    };
    fill(texts, locale, &message.params())
}

/// A ChatGPT message the crate wrote in Spanish (a plan message or a sign-in
/// error), in `locale`; unknown texts are returned as they are.
pub fn chatgpt_message(locale: Locale, message: &str) -> String {
    if let Some(plan) = PlanMessage::recognize(message) {
        return plan_message(locale, &plan);
    }
    match AuthError::recognize(message) {
        Some(err) => auth_error(locale, &err),
        None => message.to_owned(),
    }
}

/// A store error in `locale`: the profile and goal errors the user can cause
/// are translated; the others (database failures) keep their technical text.
pub fn store_error(locale: Locale, err: &StoreError) -> String {
    let texts = match err {
        StoreError::EmptyProfileName => EMPTY_PROFILE_NAME,
        StoreError::ProfileNameTooLong => {
            let max = PROFILE_NAME_MAX.to_string();
            return PROFILE_NAME_TOO_LONG.fill(locale, &[("max", &max)]);
        }
        StoreError::DuplicateProfileName => DUPLICATE_PROFILE_NAME,
        StoreError::LastProfile => LAST_PROFILE,
        StoreError::ProfileNotFound => PROFILE_NOT_FOUND,
        StoreError::InvalidGoal => INVALID_GOAL,
        other => return other.to_string(),
    };
    locale.pick(texts).to_owned()
}

/// `texts` in `locale` with owned parameter values.
fn fill(texts: Texts, locale: Locale, params: &[(&str, String)]) -> String {
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    texts.fill(locale, &params)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCALES: [Locale; 5] = [Locale::Es, Locale::En, Locale::Pt, Locale::Fr, Locale::De];

    /// The `{name}` placeholders of `text`, sorted.
    fn placeholders(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                break;
            };
            found.push(rest[start..start + len + 1].to_owned());
            rest = &rest[start + len + 1..];
        }
        found.sort();
        found
    }

    #[test]
    fn every_entry_has_the_same_placeholders_in_every_language() {
        for (name, texts) in CATALOG {
            let spanish = placeholders(texts.es);
            for locale in LOCALES {
                let text = locale.pick(*texts);
                assert!(!text.is_empty(), "{name} {locale:?}");
                assert_eq!(placeholders(text), spanish, "{name} {locale:?}: {text}");
            }
        }
    }

    #[test]
    fn catalog_names_are_unique() {
        let mut names: Vec<_> = CATALOG.iter().map(|(name, _)| *name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), CATALOG.len());
    }

    #[test]
    fn french_puts_a_non_breaking_space_before_double_punctuation() {
        for (name, texts) in CATALOG {
            let chars: Vec<char> = texts.fr.chars().collect();
            for pair in chars.windows(2) {
                if matches!(pair[1], ':' | ';' | '?' | '!') {
                    assert_eq!(pair[0], '\u{a0}', "{name}: {}", texts.fr);
                }
            }
        }
    }

    fn auth_errors() -> Vec<AuthError> {
        let s = || "x".to_owned();
        vec![
            AuthError::NoStorage,
            AuthError::Storage { message: s() },
            AuthError::DamagedFile,
            AuthError::Listener { message: s() },
            AuthError::Browser,
            AuthError::Cancelled,
            AuthError::Timeout,
            AuthError::AccessDenied,
            AuthError::Authorization { code: s() },
            AuthError::MissingCode,
            AuthError::MissingClientId,
            AuthError::ClientIdMismatch,
            AuthError::CodeExchange { detail: s() },
            AuthError::KeysUnavailable,
            AuthError::InvalidIdToken { reason: s() },
            AuthError::IdentityMismatch,
            AuthError::NotSignedIn,
            AuthError::PlanNotEnabled,
            AuthError::SessionExpired,
            AuthError::InvalidClient,
            AuthError::RefreshRefused { detail: s() },
            AuthError::Unavailable { status: 503 },
            AuthError::Network { message: s() },
            AuthError::BadResponse,
        ]
    }

    fn plan_messages() -> Vec<PlanMessage> {
        vec![
            PlanMessage::UsageLimit,
            PlanMessage::UsageUnavailable,
            PlanMessage::NotEligible,
            PlanMessage::Unsupported {
                param: "tools".into(),
            },
            PlanMessage::UnsupportedRequest,
            PlanMessage::NotAuthorized,
            PlanMessage::InvalidUser,
            PlanMessage::SessionRejected,
            PlanMessage::Forbidden,
            PlanMessage::Redirected { status: 302 },
            PlanMessage::Throttled,
            PlanMessage::Rejected {
                status: 400,
                code: "bad".into(),
            },
            PlanMessage::RejectedStatus { status: 404 },
            PlanMessage::NotSignedIn,
            PlanMessage::PlanNotEnabled,
        ]
    }

    #[test]
    fn spanish_sign_in_and_plan_texts_are_the_crate_texts() {
        for err in auth_errors() {
            assert_eq!(auth_error(Locale::Es, &err), err.to_string());
        }
        for message in plan_messages() {
            assert_eq!(plan_message(Locale::Es, &message), message.to_string());
        }
    }

    #[test]
    fn sign_in_and_plan_texts_are_translated() {
        assert_eq!(
            auth_error(Locale::En, &AuthError::SessionExpired),
            "Your ChatGPT session has expired. Sign in again"
        );
        assert_eq!(
            auth_error(Locale::En, &AuthError::DamagedFile),
            "The ChatGPT session file (chatgpt-auth.json) is damaged. Use “Sign out” to start again"
        );
        assert_eq!(
            plan_message(
                Locale::De,
                &PlanMessage::Unsupported {
                    param: "tools".into()
                }
            ),
            "ChatGPT unterstützt „tools“ in dieser Anfrage nicht. Versuche ein anderes Modell"
        );
        assert_eq!(
            chatgpt_message(Locale::En, &PlanMessage::UsageLimit.to_string()),
            "You have reached the limit of your ChatGPT plan"
        );
        assert_eq!(
            chatgpt_message(
                Locale::En,
                &AuthError::RefreshRefused {
                    detail: "invalid_grant".into()
                }
                .to_string()
            ),
            "ChatGPT did not renew the session (invalid_grant)"
        );
        assert_eq!(chatgpt_message(Locale::En, "otra cosa"), "otra cosa");
    }

    #[test]
    fn store_errors_the_user_causes_are_translated() {
        assert_eq!(
            store_error(Locale::Es, &StoreError::ProfileNameTooLong),
            StoreError::ProfileNameTooLong.to_string()
        );
        for err in [
            StoreError::EmptyProfileName,
            StoreError::DuplicateProfileName,
            StoreError::LastProfile,
            StoreError::ProfileNotFound,
            StoreError::InvalidGoal,
        ] {
            assert_eq!(store_error(Locale::Es, &err), err.to_string());
        }
        assert_eq!(
            store_error(Locale::En, &StoreError::ProfileNameTooLong),
            "The profile name can have at most 40 characters"
        );
        assert_eq!(
            store_error(Locale::De, &StoreError::LastProfile),
            "Das einzige Profil kann nicht gelöscht werden"
        );
        assert_eq!(store_error(Locale::En, &StoreError::NotFound), "not found");
    }
}
