La classe generica Exchanger<T> permette a due thread di scambiarsi un valore di tipo T. Essa offre esclusivamente il metodo pubblico T exchange( T t) che blocca il
thread chiamante senza consumare CPU fino a che un altro thread non invoca lo stesso metodo, sulla stessa istanza. Quando questo avviene, il metodo restituisce
l’oggetto passato come parametro dal thread opposto.
