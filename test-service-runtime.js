
addEventListener("serve", function (event) {
    console.log("Recieved serve request");
    event.respondWith(new ServiceResponse(true, { data: "test" }));
});
  