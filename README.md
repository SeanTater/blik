# blik: Personal photo manager
Blik is a photo manager web app (based on rphotos) with a few simple goals:
* [x] Get started as easily as possible: based on SQLite, no databases to setup
* [x] Upload from your phone or other device via a webapp
* [x] Single user authentication: each server is for one person
* [x] Collect photos into stories so you can see stuff you took together

## We also have some lofty goals:
* [ ] Caption images automatically
* [ ] Recognize faces automatically
* [ ] Share some images publically
* [ ] Allow searching other's collections (when permitted)

## Blik doesn't do some things:
* Edit or retouch photos
* Have any native application for iOS or Android or Linux or anything else
* Have any connection to a cloud provider
* Cost or bill you anything

## Status
Blik serves some of my purposes already, and it's pretty fast and stable.
However there are big things it doesn't support yet, like videos. We're working on that.
We also haven't done any of the cool gizmos yet.

## Get started

```sh
cd a/new/folder/where/you/want/your/photos
blik webserver
```

**Just running it should do the trick**
It will set up databases and configuration automatically in the current directory.
In most cases it should take a fraction of a second and even open up your web browser.
To log in the first time just use the code it prints out and you should be on your way!