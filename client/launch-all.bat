@echo off
start "BAPert Mail" cmd /k "cd /d E:\Repos\vibesql-mail\client && node bin/vibesql-mail.js --agent BAPert"
start "DotNetPert Mail" cmd /k "cd /d E:\Repos\vibesql-mail\client && node bin/vibesql-mail.js --agent DotNetPert"
start "NextPert Mail" cmd /k "cd /d E:\Repos\vibesql-mail\client && node bin/vibesql-mail.js --agent NextPert"
start "QAPert Mail" cmd /k "cd /d E:\Repos\vibesql-mail\client && node bin/vibesql-mail.js --agent QAPert"
echo All 4 agent mail TUIs launched.
