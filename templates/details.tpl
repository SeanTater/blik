<!doctype html>
<html>
  <head>
    <title>Photo details</title>
  </head>
  <body>
    <h1>Photo details</h1>
    <p>{{photo.path}}</p>
    <p><img src="/view/{{photo.id}}"></p>
    <p>Tags: {{#tags}}<a href="/tag/{{slug}}">{{tag}}</a>, {{/tags}}</p>
  </body>
</html>
