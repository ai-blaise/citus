SELECT graphql.resolve($$
  {
    mb14_customersCollection(first: 10) {
      edges {
        node {
          id
          name
          mb14_ordersCollection(first: 5) {
            edges { node { id amount } }
          }
        }
      }
    }
  }
$$);
