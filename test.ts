// adding
let resp = await fetch("http://localhost:8080/add/base", {
    method:"POST", 
    headers: {
        "Content-Type": "application/json",
        "auth":"abc123"
    },
    body: JSON.stringify(["email 1", "email 2", "wmail 3", "email 4", "email 5"]),
})
let text = await resp.text()
console.log(text)

// fetching
resp = await fetch("http://localhost:8080/fetch/base/3", {
    headers: {
        "auth":"abc123"
    },
})

text = await resp.text()
console.log(text)

export {}
