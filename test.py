import requests
import json

# Adding to database
url = "http://localhost:8080/add/base"
headers = {
    "Content-Type": "application/json",
    "auth": "abc123"
}
data = ["email 1", "email 2", "wmail 3", "email 4", "email 5"]

response = requests.post(url, headers=headers, data=json.dumps(data))
text = response.text
print(text)

# Fetching from database
url = "http://localhost:8080/fetch/base/3"
headers = {
    "auth": "abc123"
}

response = requests.get(url, headers=headers)
text = response.text
print(text)
