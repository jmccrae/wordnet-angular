angular.module('app', ['ngMaterial']);

angular.module('app').controller('WordNetController', 
        function($scope, $http) {
    $scope.synsets = [];
});

angular.module('app').controller('SearchController',
        function($scope, $http) {
    var self = this;
    self.selectedItemChange = function(item) {
        if(item) {
            $http.get("/json/lemma/"+item.item).then(
                function(result) {
                    $scope.$parent.synsets = result.data;
                }, function(response) {
                    console.log(response.data);
                });
        } else {
            $scope.$parent.synsets = [];
        }
    };

    self.querySearch = function (query) {
        return $http.get("/autocomplete/lemma/"+ query).then(
            function(result) {
                return result.data;
            }, function(response) {
                console.log(response.data);
            });
        };
});

angular.module('app').component('synset', {
        templateUrl: '/static/synset.html',
        bindings: {
            synset: '='
        }
    });
