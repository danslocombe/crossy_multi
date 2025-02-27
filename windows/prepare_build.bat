cargo build --release -F publish,demo,steam

rmdir builds\resources /S /Q
rmdir builds\shaders /S /Q

mkdir builds\resources
mkdir builds\shaders

robocopy resources builds\resources /s /e
robocopy shaders builds\shaders /s /e
robocopy dependencies builds /s /e

copy D:\crossy_multi\windows\target\release\roadtoads.exe builds